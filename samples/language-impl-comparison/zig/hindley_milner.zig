// Hindley-Milner 型推論器実装 (Zig)
// Algorithm W による単一化ベース型推論
// Zigの特徴: 明示的メモリ管理、comptime、エラーユニオン型

const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const StringHashMap = std.StringHashMap;

// 型変数カウンター
const TyVarCounter = struct {
    value: usize = 0,

    fn fresh(self: *TyVarCounter) usize {
        const id = self.value;
        self.value += 1;
        return id;
    }

    fn reset(self: *TyVarCounter) void {
        self.value = 0;
    }
};

// 型の定義
const Ty = union(enum) {
    var_: *TyVar,
    int,
    bool_,
    fun: struct { param: *Ty, result: *Ty },

    fn deinit(self: *Ty, allocator: Allocator) void {
        switch (self.*) {
            .var_ => |tv| {
                switch (tv.kind) {
                    .link => |t| t.deinit(allocator),
                    .unbound => {},
                }
                allocator.destroy(tv);
            },
            .fun => |f| {
                f.param.deinit(allocator);
                f.result.deinit(allocator);
                allocator.destroy(f.param);
                allocator.destroy(f.result);
            },
            else => {},
        }
    }
};

const TyVar = struct {
    kind: TyVarKind,

    const TyVarKind = union(enum) {
        unbound: struct { id: usize, level: i32 },
        link: *Ty,
    };
};

// 式の定義
const Expr = union(enum) {
    var_: []const u8,
    int: i64,
    bool_: bool,
    lam: struct { param: []const u8, body: *Expr },
    app: struct { func: *Expr, arg: *Expr },
    let: struct { name: []const u8, value: *Expr, body: *Expr },
    if_: struct { cond: *Expr, then_branch: *Expr, else_branch: *Expr },
    binop: struct { op: BinOp, left: *Expr, right: *Expr },
};

const BinOp = enum { add, sub, mul, eq, lt };

// 型スキーム（多相型）
const Scheme = struct {
    vars: ArrayList(usize),
    ty: *Ty,

    fn deinit(self: *Scheme, allocator: Allocator) void {
        self.vars.deinit();
        self.ty.deinit(allocator);
        allocator.destroy(self.ty);
    }
};

// 型環境
const Env = StringHashMap(*Scheme);

// エラー型
const TypeError = error{
    UnboundVariable,
    OccursCheckFailed,
    UnificationFailed,
    OutOfMemory,
};

// 型の文字列化
fn stringOfTy(ty: *const Ty, writer: anytype) !void {
    switch (ty.*) {
        .var_ => |tv| {
            switch (tv.kind) {
                .link => |t| try stringOfTy(t, writer),
                .unbound => |u| try writer.print("'t{d}", .{u.id}),
            }
        },
        .int => try writer.writeAll("Int"),
        .bool_ => try writer.writeAll("Bool"),
        .fun => |f| {
            switch (f.param.*) {
                .fun => {
                    try writer.writeAll("(");
                    try stringOfTy(f.param, writer);
                    try writer.writeAll(")");
                },
                else => try stringOfTy(f.param, writer),
            }
            try writer.writeAll(" -> ");
            try stringOfTy(f.result, writer);
        },
    }
}

// 型変数の出現チェック（無限型防止）
fn occurs(tvr: *TyVar, level: i32, ty: *const Ty) bool {
    switch (ty.*) {
        .var_ => |tv| {
            if (tvr == tv) return true;
            switch (tv.kind) {
                .unbound => |*u| {
                    const min_level = @min(level, u.level);
                    tv.kind = .{ .unbound = .{ .id = u.id, .level = min_level } };
                    return false;
                },
                .link => |t| return occurs(tvr, level, t),
            }
        },
        .fun => |f| return occurs(tvr, level, f.param) or occurs(tvr, level, f.result),
        .int, .bool_ => return false,
    }
}

// 単一化
fn unify(ty1: *const Ty, ty2: *const Ty) TypeError!void {
    switch (ty1.*) {
        .var_ => |tv1| {
            switch (tv1.kind) {
                .link => |t1| return unify(t1, ty2),
                .unbound => |u1| {
                    switch (ty2.*) {
                        .var_ => |tv2| {
                            switch (tv2.kind) {
                                .link => |t2| return unify(ty1, t2),
                                .unbound => |u2| {
                                    if (u1.id == u2.id) return;
                                    tv1.kind = .{ .link = @constCast(ty2) };
                                },
                            }
                        },
                        else => {
                            if (occurs(tv1, u1.level, ty2)) {
                                return TypeError.OccursCheckFailed;
                            }
                            tv1.kind = .{ .link = @constCast(ty2) };
                        },
                    }
                },
            }
        },
        .int => {
            if (ty2.* != .int) return TypeError.UnificationFailed;
        },
        .bool_ => {
            if (ty2.* != .bool_) return TypeError.UnificationFailed;
        },
        .fun => |f1| {
            switch (ty2.*) {
                .var_ => return unify(ty2, ty1),
                .fun => |f2| {
                    try unify(f1.param, f2.param);
                    try unify(f1.result, f2.result);
                },
                else => return TypeError.UnificationFailed,
            }
        },
    }
}

// 型の一般化（多相化）
fn generalize(allocator: Allocator, level: i32, ty: *Ty) !*Scheme {
    var vars = ArrayList(usize).init(allocator);
    try collectVars(ty, level, &vars);

    // 重複除去とソート
    const sorted = try vars.toOwnedSlice();
    defer allocator.free(sorted);
    std.mem.sort(usize, sorted, {}, comptime std.sort.asc(usize));

    var unique = ArrayList(usize).init(allocator);
    var prev: ?usize = null;
    for (sorted) |id| {
        if (prev == null or prev.? != id) {
            try unique.append(id);
            prev = id;
        }
    }

    const scheme = try allocator.create(Scheme);
    scheme.* = .{ .vars = unique, .ty = ty };
    return scheme;
}

fn collectVars(ty: *const Ty, level: i32, vars: *ArrayList(usize)) !void {
    switch (ty.*) {
        .var_ => |tv| {
            switch (tv.kind) {
                .unbound => |u| {
                    if (u.level > level) {
                        try vars.append(u.id);
                    }
                },
                .link => |t| try collectVars(t, level, vars),
            }
        },
        .fun => |f| {
            try collectVars(f.param, level, vars);
            try collectVars(f.result, level, vars);
        },
        else => {},
    }
}

// 型の具体化（多相型のインスタンス化）
fn instantiate(allocator: Allocator, counter: *TyVarCounter, level: i32, scheme: *const Scheme) !*Ty {
    var subst = std.AutoHashMap(usize, *Ty).init(allocator);
    defer {
        var iter = subst.valueIterator();
        while (iter.next()) |ty_ptr| {
            // substの値は後でapplyで使われるのでここでは解放しない
        }
        subst.deinit();
    }

    for (scheme.vars.items) |id| {
        const tv = try allocator.create(TyVar);
        tv.* = .{ .kind = .{ .unbound = .{ .id = counter.fresh(), .level = level } } };
        const new_ty = try allocator.create(Ty);
        new_ty.* = .{ .var_ = tv };
        try subst.put(id, new_ty);
    }

    return try applySubst(allocator, &subst, scheme.ty);
}

fn applySubst(allocator: Allocator, subst: *const std.AutoHashMap(usize, *Ty), ty: *const Ty) TypeError!*Ty {
    switch (ty.*) {
        .var_ => |tv| {
            switch (tv.kind) {
                .unbound => |u| {
                    if (subst.get(u.id)) |new_ty| {
                        const result = try allocator.create(Ty);
                        result.* = new_ty.*;
                        return result;
                    }
                    const result = try allocator.create(Ty);
                    result.* = ty.*;
                    return result;
                },
                .link => |t| return applySubst(allocator, subst, t),
            }
        },
        .fun => |f| {
            const param = try applySubst(allocator, subst, f.param);
            const result = try applySubst(allocator, subst, f.result);
            const new_ty = try allocator.create(Ty);
            new_ty.* = .{ .fun = .{ .param = param, .result = result } };
            return new_ty;
        },
        else => {
            const result = try allocator.create(Ty);
            result.* = ty.*;
            return result;
        },
    }
}

// 型推論（Algorithm W）
fn infer(
    allocator: Allocator,
    counter: *TyVarCounter,
    env: *const Env,
    level: i32,
    expr: *const Expr,
) TypeError!*Ty {
    switch (expr.*) {
        .var_ => |name| {
            if (env.get(name)) |scheme| {
                return instantiate(allocator, counter, level, scheme);
            }
            return TypeError.UnboundVariable;
        },
        .int => {
            const ty = try allocator.create(Ty);
            ty.* = .int;
            return ty;
        },
        .bool_ => {
            const ty = try allocator.create(Ty);
            ty.* = .bool_;
            return ty;
        },
        .lam => |lam| {
            const tv = try allocator.create(TyVar);
            tv.* = .{ .kind = .{ .unbound = .{ .id = counter.fresh(), .level = level } } };
            const param_ty = try allocator.create(Ty);
            param_ty.* = .{ .var_ = tv };

            const param_scheme = try allocator.create(Scheme);
            param_scheme.* = .{ .vars = ArrayList(usize).init(allocator), .ty = param_ty };

            var new_env = Env.init(allocator);
            defer new_env.deinit();
            var iter = env.iterator();
            while (iter.next()) |entry| {
                try new_env.put(entry.key_ptr.*, entry.value_ptr.*);
            }
            try new_env.put(lam.param, param_scheme);

            const body_ty = try infer(allocator, counter, &new_env, level, lam.body);

            const result = try allocator.create(Ty);
            result.* = .{ .fun = .{ .param = param_ty, .result = body_ty } };
            return result;
        },
        .app => |app| {
            const func_ty = try infer(allocator, counter, env, level, app.func);
            const arg_ty = try infer(allocator, counter, env, level, app.arg);

            const tv = try allocator.create(TyVar);
            tv.* = .{ .kind = .{ .unbound = .{ .id = counter.fresh(), .level = level } } };
            const result_ty = try allocator.create(Ty);
            result_ty.* = .{ .var_ = tv };

            const expected_param = try allocator.create(Ty);
            expected_param.* = arg_ty.*;
            const expected_result = try allocator.create(Ty);
            expected_result.* = result_ty.*;
            const expected = try allocator.create(Ty);
            expected.* = .{ .fun = .{ .param = expected_param, .result = expected_result } };

            try unify(func_ty, expected);
            return result_ty;
        },
        .let => |let_| {
            const value_ty = try infer(allocator, counter, env, level + 1, let_.value);
            const value_scheme = try generalize(allocator, level, value_ty);

            var new_env = Env.init(allocator);
            defer new_env.deinit();
            var iter = env.iterator();
            while (iter.next()) |entry| {
                try new_env.put(entry.key_ptr.*, entry.value_ptr.*);
            }
            try new_env.put(let_.name, value_scheme);

            return infer(allocator, counter, &new_env, level, let_.body);
        },
        .if_ => |if_expr| {
            const cond_ty = try infer(allocator, counter, env, level, if_expr.cond);
            const bool_ty = try allocator.create(Ty);
            bool_ty.* = .bool_;
            try unify(cond_ty, bool_ty);

            const then_ty = try infer(allocator, counter, env, level, if_expr.then_branch);
            const else_ty = try infer(allocator, counter, env, level, if_expr.else_branch);
            try unify(then_ty, else_ty);

            return then_ty;
        },
        .binop => |binop| {
            const t1 = try infer(allocator, counter, env, level, binop.left);
            const t2 = try infer(allocator, counter, env, level, binop.right);

            const int_ty = try allocator.create(Ty);
            int_ty.* = .int;

            switch (binop.op) {
                .add, .sub, .mul => {
                    try unify(t1, int_ty);
                    try unify(t2, int_ty);
                    const result = try allocator.create(Ty);
                    result.* = .int;
                    return result;
                },
                .eq, .lt => {
                    try unify(t1, int_ty);
                    try unify(t2, int_ty);
                    const result = try allocator.create(Ty);
                    result.* = .bool_;
                    return result;
                },
            }
        },
    }
}

// トップレベル推論
fn inferExpr(allocator: Allocator, expr: *const Expr) !*Scheme {
    var counter = TyVarCounter{};
    var env = Env.init(allocator);
    defer env.deinit();

    const ty = try infer(allocator, &counter, &env, 0, expr);
    return generalize(allocator, -1, ty);
}

// テスト実行
pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const stdout = std.io.getStdOut().writer();
    try stdout.writeAll("=== Hindley-Milner Type Inference Tests (Zig) ===\n");

    // テスト: int literal
    {
        const expr = Expr{ .int = 42 };
        try test_(allocator, stdout, "int literal", &expr, "Int");
    }

    // テスト: bool literal
    {
        const expr = Expr{ .bool_ = true };
        try test_(allocator, stdout, "bool literal", &expr, "Bool");
    }

    // テスト: identity function
    {
        const body = try allocator.create(Expr);
        body.* = .{ .var_ = "x" };
        const expr = Expr{ .lam = .{ .param = "x", .body = body } };
        try test_(allocator, stdout, "identity", &expr, "'t0 -> 't0");
    }

    // テスト: application
    {
        const lam_body = try allocator.create(Expr);
        lam_body.* = .{ .var_ = "x" };
        const lam = try allocator.create(Expr);
        lam.* = .{ .lam = .{ .param = "x", .body = lam_body } };
        const arg = try allocator.create(Expr);
        arg.* = .{ .int = 42 };
        const expr = Expr{ .app = .{ .func = lam, .arg = arg } };
        try test_(allocator, stdout, "application", &expr, "Int");
    }

    try stdout.writeAll("\nAll tests completed.\n");
}

fn test_(allocator: Allocator, writer: anytype, name: []const u8, expr: *const Expr, expected: []const u8) !void {
    var result = inferExpr(allocator, expr) catch |err| {
        try writer.print("ERROR: {s} : {}\n", .{ name, err });
        return;
    };
    defer result.deinit(allocator);

    var buf = std.ArrayList(u8).init(allocator);
    defer buf.deinit();
    try stringOfTy(result.ty, buf.writer());

    const ty_str = buf.items;
    if (std.mem.eql(u8, ty_str, expected)) {
        try writer.print("PASS: {s} : {s}\n", .{ name, ty_str });
    } else {
        try writer.print("FAIL: {s} : {s} (expected: {s})\n", .{ name, ty_str, expected });
    }
}
