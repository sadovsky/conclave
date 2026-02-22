/// A complete parsed module (one `.conclave` file).
///
/// After normalization:
/// - `types`, `capabilities`, `intrinsics` are sorted by `name`/`alias`.
/// - All signatures are in canonical form: `fetch(Url)->Html` (no whitespace).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub version: String,
    pub types: Vec<TypeDecl>,
    pub capabilities: Vec<CapDecl>,
    pub intrinsics: Vec<IntrinsicDecl>,
    pub goals: Vec<GoalDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeDecl {
    pub name: String,
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint: Option<TypeConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeConstraint {
    pub validator: String, // e.g. "re2"
    pub pattern: String,
}

/// A declared capability (external side-effectful call).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CapDecl {
    pub alias: String,
    /// Normalized: `"fetch(Url)->Html"` — no whitespace.
    pub signature: String,
}

/// A declared intrinsic (pure built-in function).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntrinsicDecl {
    pub alias: String,
    /// Normalized: `"assemble_json(List<String>)->Json"` — no whitespace.
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GoalDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub returns: String,
    pub want: WantBlock,
    pub constraints: Vec<ConstraintExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WantBlock {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum Stmt {
    Let {
        name: String,
        expr: Expr,
    },
    Map {
        list: String,
        binder: String,
        body: Vec<Stmt>,
    },
    Emit {
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum Expr {
    Ident { name: String },
    StringLit { value: String },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConstraintExpr {
    pub op: CmpOp,
    pub left: ConstraintLeft,
    pub right: ConstraintValue,
}

/// Left-hand side of a constraint expression.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum ConstraintLeft {
    /// A dotted path like `determinism.mode` or `scheduler.max_inflight`.
    Path { segments: Vec<String> },
    /// A function call like `rate_limit(fetch)`.
    FnCall { name: String, args: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum ConstraintValue {
    Number { value: u64 },
    Rate { value: u64, unit: String }, // "req/s"
    StringLit { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Eq,
    LtEq,
}
