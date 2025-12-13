use criterion::{criterion_group, criterion_main, Criterion};
use reml_runtime::parse::{chainl1, ok, run, symbol, Parser, Reply};
use reml_runtime::run_config::RunConfig;

fn digit() -> Parser<i64> {
    Parser::new(|state| {
        let input = state.input().clone();
        match input.remaining().chars().next() {
            Some(ch) if ch.is_ascii_digit() => {
                let rest = input.advance(ch.len_utf8());
                Reply::Ok {
                    value: ch.to_digit(10).unwrap() as i64,
                    span: input.span_to(&rest),
                    consumed: true,
                    rest,
                }
            }
            _ => Reply::Err {
                error: reml_runtime::parse::ParseError::new("digit expected", input.position()),
                consumed: false,
                committed: false,
            },
        }
    })
}

fn factor() -> Parser<i64> {
    digit()
}

fn term() -> Parser<i64> {
    let mul = symbol(None, "*").map(|_| |l: i64, r: i64| l * r);
    factor().chainl1(mul)
}

fn expr() -> Parser<i64> {
    let add = symbol(None, "+").map(|_| |l: i64, r: i64| l + r);
    term().chainl1(add)
}

fn expr_parser() -> Parser<i64> {
    expr().skip_r(ok(()))
}

fn bench_parse_profile(c: &mut Criterion) {
    let parser = expr_parser();
    let input = "1+2*3+4*5+6+7*8+9";
    let mut base_cfg = RunConfig::default();
    base_cfg.packrat = true;
    base_cfg.require_eof = true;
    let mut profile_cfg = base_cfg.clone();
    profile_cfg.profile = true;

    let mut group = c.benchmark_group("parse::profile");
    group.bench_function("packrat_only", |b| {
        b.iter(|| {
            let _ = run(&parser, input, &base_cfg);
        });
    });
    group.bench_function("packrat_with_profile", |b| {
        b.iter(|| {
            let _ = run(&parser, input, &profile_cfg);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_parse_profile);
criterion_main!(benches);
