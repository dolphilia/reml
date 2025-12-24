use crate::target_machine::DataLayoutSpec;

/// Reml 型の簡易列挙。最小限の構造体と ADT を扱う。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemlType {
    Bool,
    I32,
    I64,
    F64,
    Pointer,
    String,
    Slice(Box<RemlType>),
    Ref { mutable: bool, to: Box<RemlType> },
    Unit,
    RowTuple(Vec<RemlType>),
    Adt {
        tag_bits: u32,
        variants: Vec<RemlType>,
    },
}

/// レイアウト情報。
#[derive(Clone, Debug)]
pub struct TypeLayout {
    pub size: u64,
    pub align: u64,
    pub description: String,
}

/// TypeMappingContext は DataLayout との整合性を保ちながら Reml 型を LLVM 型へ丸める目的のコンテキスト。
#[derive(Clone, Debug)]
pub struct TypeMappingContext {
    data_layout: DataLayoutSpec,
}

impl TypeMappingContext {
    pub fn new(data_layout: DataLayoutSpec) -> Self {
        Self { data_layout }
    }

    pub fn data_layout(&self) -> &DataLayoutSpec {
        &self.data_layout
    }

    /// Reml 型に対応する LLVM 型のサイズ/アラインメントを概算して返す。
    pub fn layout_of(&self, ty: &RemlType) -> TypeLayout {
        match ty {
            RemlType::Bool => TypeLayout {
                size: 1,
                align: 1,
                description: "i1".into(),
            },
            RemlType::I32 => TypeLayout {
                size: 4,
                align: 4,
                description: "i32".into(),
            },
            RemlType::I64 => TypeLayout {
                size: 8,
                align: 8,
                description: "i64".into(),
            },
            RemlType::F64 => TypeLayout {
                size: 8,
                align: 8,
                description: "double".into(),
            },
            RemlType::Pointer => TypeLayout {
                size: 8,
                align: 8,
                description: "ptr".into(),
            },
            RemlType::String => TypeLayout {
                size: 16,
                align: 8,
                description: "{i8*, i64}".into(),
            },
            RemlType::Slice(_) => TypeLayout {
                size: 16,
                align: 8,
                description: "{ptr, i64}".into(),
            },
            RemlType::Ref { .. } => TypeLayout {
                size: 8,
                align: 8,
                description: "ptr".into(),
            },
            RemlType::Unit => TypeLayout {
                size: 0,
                align: 1,
                description: "ptr".into(),
            },
            RemlType::RowTuple(fields) => {
                let mut size = 0;
                let mut align = 1;
                for field in fields {
                    let layout = self.layout_of(field);
                    align = align.max(layout.align);
                    size = ((size + layout.align - 1) / layout.align) * layout.align + layout.size;
                }
                TypeLayout {
                    size,
                    align,
                    description: format!("tuple[{}]", fields.len()),
                }
            }
            RemlType::Adt { tag_bits, variants } => {
                let tag_size = (*tag_bits + 7) / 8;
                let mut max_variant = 0;
                for variant in variants {
                    let layout = self.layout_of(variant);
                    max_variant = max_variant.max(layout.size);
                }
                let payload = max_variant;
                TypeLayout {
                    size: payload + tag_size as u64,
                    align: 8,
                    description: "adt".into(),
                }
            }
        }
    }
}
