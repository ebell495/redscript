use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::iter;
use std::ops::{Add, Sub};

use enum_as_inner::EnumAsInner;
use itertools::Itertools;
use strum::{Display, EnumString, IntoStaticStr};

use crate::Ref;

#[derive(Debug, EnumAsInner)]
pub enum Expr<Name: NameKind>
where
    Name: NameKind,
    Name::Reference: Debug,
    Name::Callable: Debug,
    Name::Local: Debug,
    Name::Function: Debug,
    Name::Member: Debug,
    Name::Type: Debug,
{
    Ident(Name::Reference, Span),
    Constant(Constant, Span),
    ArrayLit(Vec<Self>, Option<Name::Type>, Span),
    InterpolatedString(Ref<str>, Vec<(Self, Ref<str>)>, Span),
    Declare(Name::Local, Option<Name::Type>, Option<Box<Self>>, Span),
    Cast(Name::Type, Box<Self>, Span),
    Assign(Box<Self>, Box<Self>, Span),
    Call(Name::Callable, Vec<Name::Type>, Vec<Self>, Span),
    MethodCall(Box<Self>, Name::Function, Vec<Self>, Span),
    Member(Box<Self>, Name::Member, Span),
    ArrayElem(Box<Self>, Box<Self>, Span),
    New(Name::Type, Vec<Self>, Span),
    Return(Option<Box<Self>>, Span),
    Seq(Seq<Name>),
    Switch(Box<Self>, Vec<SwitchCase<Name>>, Option<Seq<Name>>, Span),
    Goto(Target, Span),
    If(Box<Self>, Seq<Name>, Option<Seq<Name>>, Span),
    Conditional(Box<Self>, Box<Self>, Box<Self>, Span),
    While(Box<Self>, Seq<Name>, Span),
    ForIn(Name::Local, Box<Self>, Seq<Name>, Span),
    BinOp(Box<Self>, Box<Self>, BinOp, Span),
    UnOp(Box<Self>, UnOp, Span),
    This(Span),
    Super(Span),
    Break(Span),
    Null(Span),
}

pub trait NameKind {
    type Reference;
    type Callable;
    type Local;
    type Function;
    type Member;
    type Type;
}

#[derive(Debug)]
pub struct SourceAst;

impl NameKind for SourceAst {
    type Reference = Ident;
    type Callable = Ident;
    type Local = Ident;
    type Function = Ident;
    type Member = Ident;
    type Type = TypeName;
}

impl<N> Expr<N>
where
    N: NameKind,
    N::Reference: Debug,
    N::Callable: Debug,
    N::Local: Debug,
    N::Function: Debug,
    N::Member: Debug,
    N::Type: Debug,
{
    pub const EMPTY: Self = Expr::Seq(Seq { exprs: vec![] });

    pub fn is_empty(&self) -> bool {
        match self {
            Expr::Seq(seq) => seq.exprs.iter().all(|expr| expr.is_empty()),
            Expr::Goto(target, _) => target.resolved,
            _ => false,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Expr::Ident(_, span) => *span,
            Expr::Constant(_, span) => *span,
            Expr::ArrayLit(_, _, span) => *span,
            Expr::InterpolatedString(_, _, span) => *span,
            Expr::Declare(_, _, _, span) => *span,
            Expr::Cast(_, _, span) => *span,
            Expr::Assign(_, _, span) => *span,
            Expr::Call(_, _, _, span) => *span,
            Expr::MethodCall(_, _, _, span) => *span,
            Expr::Member(_, _, span) => *span,
            Expr::ArrayElem(_, _, span) => *span,
            Expr::New(_, _, span) => *span,
            Expr::Return(_, span) => *span,
            Expr::Seq(seq) => {
                let start = seq.exprs.first().map(Self::span).unwrap_or_default();
                let end = seq.exprs.last().map(Self::span).unwrap_or_default();
                start.merge(end)
            }
            Expr::Switch(_, _, _, span) => *span,
            Expr::Goto(_, span) => *span,
            Expr::If(_, _, _, span) => *span,
            Expr::Conditional(_, _, _, span) => *span,
            Expr::While(_, _, span) => *span,
            Expr::ForIn(_, _, _, span) => *span,
            Expr::BinOp(_, _, _, span) => *span,
            Expr::UnOp(_, _, span) => *span,
            Expr::This(span) => *span,
            Expr::Super(span) => *span,
            Expr::Break(span) => *span,
            Expr::Null(span) => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    String(Literal, Ref<str>),
    F32(f32),
    F64(f64),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    Bool(bool),
}

#[derive(Debug, Clone, Eq, PartialOrd, Ord)]
pub enum Ident {
    Static(&'static str),
    Owned(Ref<str>),
}

impl Ident {
    pub fn new(str: String) -> Ident {
        Ident::Owned(Ref::from(str))
    }

    pub fn to_owned(&self) -> Ref<str> {
        match self {
            Ident::Static(str) => Ref::from(*str),
            Ident::Owned(rc) => rc.clone(),
        }
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ident::Static(str) => f.write_str(str),
            Ident::Owned(str) => f.write_str(str),
        }
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        match self {
            Ident::Static(str) => str,
            Ident::Owned(rc) => rc,
        }
    }
}

impl From<Ref<str>> for Ident {
    fn from(rc: Ref<str>) -> Self {
        Self::Owned(rc)
    }
}

#[derive(Debug, Clone, Copy, Display, EnumString, IntoStaticStr)]
pub enum BinOp {
    #[strum(serialize = "OperatorAssignAdd")]
    AssignAdd,
    #[strum(serialize = "OperatorAssignSubtract")]
    AssignSubtract,
    #[strum(serialize = "OperatorAssignMultiply")]
    AssignMultiply,
    #[strum(serialize = "OperatorAssignDivide")]
    AssignDivide,
    #[strum(serialize = "OperatorAssignOr")]
    AssignOr,
    #[strum(serialize = "OperatorAssignAnd")]
    AssignAnd,
    #[strum(serialize = "OperatorLogicOr")]
    LogicOr,
    #[strum(serialize = "OperatorLogicAnd")]
    LogicAnd,
    #[strum(serialize = "OperatorOr")]
    Or,
    #[strum(serialize = "OperatorXor")]
    Xor,
    #[strum(serialize = "OperatorAnd")]
    And,
    #[strum(serialize = "OperatorEqual")]
    Equal,
    #[strum(serialize = "OperatorNotEqual")]
    NotEqual,
    #[strum(serialize = "OperatorLess")]
    Less,
    #[strum(serialize = "OperatorLessEqual")]
    LessEqual,
    #[strum(serialize = "OperatorGreater")]
    Greater,
    #[strum(serialize = "OperatorGreaterEqual")]
    GreaterEqual,
    #[strum(serialize = "OperatorAdd")]
    Add,
    #[strum(serialize = "OperatorSubtract")]
    Subtract,
    #[strum(serialize = "OperatorMultiply")]
    Multiply,
    #[strum(serialize = "OperatorDivide")]
    Divide,
    #[strum(serialize = "OperatorModulo")]
    Modulo,
}

impl BinOp {
    pub fn precedence(self) -> usize {
        match self {
            BinOp::AssignAdd => 10,
            BinOp::AssignSubtract => 10,
            BinOp::AssignMultiply => 10,
            BinOp::AssignDivide => 10,
            BinOp::AssignOr => 10,
            BinOp::AssignAnd => 10,
            BinOp::LogicOr => 9,
            BinOp::LogicAnd => 8,
            BinOp::Or => 7,
            BinOp::Xor => 6,
            BinOp::And => 5,
            BinOp::Equal => 4,
            BinOp::NotEqual => 4,
            BinOp::Less => 3,
            BinOp::LessEqual => 3,
            BinOp::Greater => 3,
            BinOp::GreaterEqual => 3,
            BinOp::Add => 2,
            BinOp::Subtract => 2,
            BinOp::Multiply => 1,
            BinOp::Divide => 1,
            BinOp::Modulo => 1,
        }
    }

    pub fn associative(self) -> bool {
        match self {
            BinOp::AssignAdd => false,
            BinOp::AssignSubtract => false,
            BinOp::AssignMultiply => false,
            BinOp::AssignDivide => false,
            BinOp::AssignOr => false,
            BinOp::AssignAnd => false,
            BinOp::LogicOr => true,
            BinOp::LogicAnd => true,
            BinOp::Or => true,
            BinOp::Xor => true,
            BinOp::And => true,
            BinOp::Equal => false,
            BinOp::NotEqual => false,
            BinOp::Less => false,
            BinOp::LessEqual => false,
            BinOp::Greater => false,
            BinOp::GreaterEqual => false,
            BinOp::Add => true,
            BinOp::Subtract => true,
            BinOp::Multiply => true,
            BinOp::Divide => false,
            BinOp::Modulo => false,
        }
    }

    pub fn does_associate(self, parent: BinOp) -> bool {
        parent.precedence() > self.precedence() || (parent.precedence() == self.precedence() && parent.associative())
    }
}

#[derive(Debug, Clone, Copy, Display, EnumString, IntoStaticStr)]
pub enum UnOp {
    #[strum(serialize = "OperatorBitNot")]
    BitNot,
    #[strum(serialize = "OperatorLogicNot")]
    LogicNot,
    #[strum(serialize = "OperatorNeg")]
    Neg,
}

#[derive(Debug)]
pub struct SwitchCase<N>
where
    N: NameKind,
    N::Reference: Debug,
    N::Callable: Debug,
    N::Local: Debug,
    N::Function: Debug,
    N::Member: Debug,
    N::Type: Debug,
{
    pub matcher: Expr<N>,
    pub body: Seq<N>,
}

#[derive(Debug)]
pub struct Seq<N>
where
    N: NameKind,
    N::Reference: Debug,
    N::Callable: Debug,
    N::Local: Debug,
    N::Function: Debug,
    N::Member: Debug,
    N::Type: Debug,
{
    pub exprs: Vec<Expr<N>>,
}

impl<N> Seq<N>
where
    N: NameKind,
    N::Reference: Debug,
    N::Callable: Debug,
    N::Local: Debug,
    N::Function: Debug,
    N::Member: Debug,
    N::Type: Debug,
{
    pub fn new(exprs: Vec<Expr<N>>) -> Seq<N> {
        Seq { exprs }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    String,
    Name,
    Resource,
    TweakDbId,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos(pub u32);

impl Pos {
    pub const ZERO: Pos = Pos(0);

    pub fn new(n: usize) -> Self {
        Pos(n as u32)
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<usize> for Pos {
    type Output = Pos;

    #[inline]
    fn add(self, rhs: usize) -> Pos {
        Pos(self.0 + rhs as u32)
    }
}

impl Sub<usize> for Pos {
    type Output = Pos;

    #[inline]
    fn sub(self, rhs: usize) -> Pos {
        Pos(self.0 - rhs as u32)
    }
}

impl From<Pos> for usize {
    #[inline]
    fn from(pos: Pos) -> Self {
        pos.0 as usize
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    pub low: Pos,
    pub high: Pos,
}

impl Span {
    pub const ZERO: Span = Span::new(Pos::ZERO, Pos::ZERO);

    pub const fn new(low: Pos, high: Pos) -> Self {
        Self { low, high }
    }

    pub fn merge(&self, other: Span) -> Span {
        Span::new(self.low.min(other.low), self.high.max(other.high))
    }

    pub fn contains(&self, pos: Pos) -> bool {
        self.low <= pos && self.high > pos
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub position: u16,
    pub resolved: bool,
}

impl Target {
    pub fn new(position: u16) -> Target {
        Target {
            position,
            resolved: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeName {
    name: Ident,
    arguments: Vec<TypeName>,
}

impl TypeName {
    pub const BOOL: Self = TypeName::basic("Bool");
    pub const INT8: Self = TypeName::basic("Int8");
    pub const INT16: Self = TypeName::basic("Int16");
    pub const INT32: Self = TypeName::basic("Int32");
    pub const INT64: Self = TypeName::basic("Int64");
    pub const UINT8: Self = TypeName::basic("Uint8");
    pub const UINT16: Self = TypeName::basic("Uint16");
    pub const UINT32: Self = TypeName::basic("Uint32");
    pub const UINT64: Self = TypeName::basic("Uint64");
    pub const FLOAT: Self = TypeName::basic("Float");
    pub const DOUBLE: Self = TypeName::basic("Double");
    pub const STRING: Self = TypeName::basic("String");
    pub const VARIANT: Self = TypeName::basic("Variant");
    pub const CNAME: Self = TypeName::basic("CName");
    pub const RESOURCE: Self = TypeName::basic("ResRef");
    pub const TWEAKDB_ID: Self = TypeName::basic("TweakDBID");
    pub const VOID: Self = TypeName::basic("Void");

    #[inline]
    pub const fn new(name: Ident, arguments: Vec<TypeName>) -> Self {
        TypeName { name, arguments }
    }

    #[inline]
    pub const fn basic(name: &'static str) -> Self {
        TypeName::new(Ident::Static(name), vec![])
    }

    #[inline]
    pub fn basic_owned(name: Ref<str>) -> Self {
        TypeName::new(Ident::Owned(name), vec![])
    }

    #[inline]
    pub fn name(&self) -> Ident {
        self.name.clone()
    }

    #[inline]
    pub fn arguments(&self) -> &[TypeName] {
        &self.arguments
    }

    pub fn kind(&self) -> Kind {
        match self.name.as_ref() {
            "ref" => Kind::Ref,
            "wref" => Kind::WRef,
            "script_ref" => Kind::ScriptRef,
            "array" => Kind::Array,
            _ => Kind::Prim,
        }
    }

    fn unwrapped(&self) -> &Self {
        match self.kind() {
            Kind::Ref => self.arguments[0].unwrapped(),
            Kind::WRef => self.arguments[0].unwrapped(),
            _ => self,
        }
    }

    // Used for identifying functions
    pub fn mangled(&self) -> Ident {
        let unwrapped = self.unwrapped();
        if unwrapped.arguments.is_empty() {
            unwrapped.name.clone()
        } else {
            let args = unwrapped.arguments.iter().map(TypeName::mangled).format(",");
            Ident::new(format!("{}<{args}>", unwrapped.name))
        }
    }

    pub fn pretty(&self) -> Ident {
        if self.arguments.is_empty() {
            self.name.clone()
        } else {
            let args = self.arguments.iter().map(TypeName::pretty).format(", ");
            Ident::new(format!("{}<{args}>", self.name))
        }
    }

    // Used for storing types in the constant pool
    pub fn repr(&self) -> Ident {
        if self.arguments.is_empty() {
            self.name.clone()
        } else {
            let str = iter::once(self.name.clone())
                .chain(self.arguments.iter().map(TypeName::repr))
                .join(":");
            Ident::new(str)
        }
    }

    pub fn from_repr(str: &str) -> TypeName {
        let mut parts = str.split(':');
        Self::from_parts(parts.next().unwrap(), parts).unwrap()
    }

    fn from_parts<'a>(name: &'a str, mut parts: impl Iterator<Item = &'a str>) -> Option<TypeName> {
        let name = Ref::from(name);
        match parts.next() {
            Some(tail) => {
                let arg = Self::from_parts(tail, parts)?;
                let type_ = TypeName {
                    name: Ident::Owned(name),
                    arguments: vec![arg],
                };
                Some(type_)
            }
            None => Some(TypeName::basic_owned(name)),
        }
    }
}

impl Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mangled().as_ref())
    }
}

#[derive(Debug)]
pub enum Kind {
    Prim,
    Ref,
    WRef,
    ScriptRef,
    Array,
}
