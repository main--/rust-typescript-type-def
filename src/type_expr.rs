use crate::emit::{Emit, EmitCtx};
use std::{io, io::Write};

#[derive(Debug, Clone, Copy)]
pub enum TypeInfo {
    Native(NativeTypeInfo),
    Custom(CustomTypeInfo),
}

#[derive(Debug, Clone, Copy)]
pub struct NativeTypeInfo {
    pub r#ref: &'static TypeExpr,
}

#[derive(Debug, Clone, Copy)]
// TODO: better name
pub struct CustomTypeInfo {
    pub name: &'static TypeName,
    pub def: &'static TypeExpr,
}

#[derive(Debug, Clone, Copy)]
pub enum TypeExpr {
    Ref(TypeInfo),
    TypeName(TypeName),
    String(TypeString),
    Tuple(Tuple),
    Object(Object),
    Array(Array),
    Union(Union),
    Intersection(Intersection),
}

#[derive(Debug, Clone, Copy)]
pub struct TypeName {
    pub path: &'static List<Ident>,
    pub name: &'static Ident,
    pub generics: &'static List<TypeExpr>,
}

#[derive(Debug, Clone, Copy)]
pub struct TypeString(pub &'static str);

#[derive(Debug, Clone, Copy)]
pub struct Tuple(pub &'static List<TypeExpr>);

#[derive(Debug, Clone, Copy)]
pub struct Object(pub &'static List<ObjectField>);

#[derive(Debug, Clone, Copy)]
pub struct ObjectField {
    pub name: &'static TypeString,
    pub optional: bool,
    pub r#type: &'static TypeExpr,
}

#[derive(Debug, Clone, Copy)]
pub struct Array(pub &'static TypeExpr);

#[derive(Debug, Clone, Copy)]
pub struct Union(pub &'static List<TypeExpr>);

#[derive(Debug, Clone, Copy)]
pub struct Intersection(pub &'static List<TypeExpr>);

#[derive(Debug, Clone, Copy)]
pub struct Ident(pub &'static str);

pub type List<T> = [&'static T];

impl TypeExpr {
    pub const fn ident(ident: &'static Ident) -> Self {
        Self::TypeName(TypeName::ident(ident))
    }
}

impl TypeName {
    pub const fn ident(ident: &'static Ident) -> Self {
        Self {
            path: &[],
            name: ident,
            generics: &[],
        }
    }
}

impl Emit for TypeExpr {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        match self {
            TypeExpr::Ref(type_info) => match type_info {
                TypeInfo::Native(NativeTypeInfo { r#ref }) => r#ref.emit(ctx),
                TypeInfo::Custom(CustomTypeInfo { name, def: _ }) => {
                    write!(ctx, "types.")?;
                    name.emit(ctx)
                },
            },
            TypeExpr::TypeName(type_name) => type_name.emit(ctx),
            TypeExpr::String(type_string) => type_string.emit(ctx),
            TypeExpr::Tuple(tuple) => tuple.emit(ctx),
            TypeExpr::Object(object) => object.emit(ctx),
            TypeExpr::Array(array) => array.emit(ctx),
            TypeExpr::Union(r#union) => r#union.emit(ctx),
            TypeExpr::Intersection(intersection) => intersection.emit(ctx),
        }
    }
}

impl Emit for TypeName {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self {
            path,
            name,
            generics,
        } = self;
        for path_part in *path {
            path_part.emit(ctx)?;
            write!(ctx, ".")?;
        }
        name.emit(ctx)?;
        if !generics.is_empty() {
            write!(ctx, "<")?;
            let mut first = true;
            for generic in *generics {
                if !first {
                    write!(ctx, ",")?;
                }
                generic.emit(ctx)?;
                first = false;
            }
            write!(ctx, ">")?;
        }
        Ok(())
    }
}

impl Emit for TypeString {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(s) = self;
        write!(ctx, "{:?}", s)?;
        Ok(())
    }
}

impl Emit for Tuple {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(items) = self;
        write!(ctx, "[")?;
        let mut first = true;
        for item in *items {
            if !first {
                write!(ctx, ",")?;
            }
            item.emit(ctx)?;
            first = false;
        }
        write!(ctx, "]")?;
        Ok(())
    }
}

impl Emit for Object {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(fields) = self;
        write!(ctx, "{{")?;
        for ObjectField {
            name,
            optional,
            r#type,
        } in *fields
        {
            name.emit(ctx)?;
            if *optional {
                write!(ctx, "?")?;
            }
            write!(ctx, ":")?;
            r#type.emit(ctx)?;
            write!(ctx, ";")?;
        }
        write!(ctx, "}}")?;
        Ok(())
    }
}

impl Emit for Array {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(item) = self;
        write!(ctx, "(")?;
        item.emit(ctx)?;
        write!(ctx, ")[]")?;
        Ok(())
    }
}

impl Emit for Union {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(branches) = self;
        if branches.is_empty() {
            write!(ctx, "never")?;
        } else {
            write!(ctx, "(")?;
            let mut first = true;
            for branch in *branches {
                if !first {
                    write!(ctx, "|")?;
                }
                branch.emit(ctx)?;
                first = false;
            }
            write!(ctx, ")")?;
        }
        Ok(())
    }
}

impl Emit for Intersection {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(members) = self;
        if members.is_empty() {
            write!(ctx, "any")?;
        } else {
            write!(ctx, "(")?;
            let mut first = true;
            for member in *members {
                if !first {
                    write!(ctx, "&")?;
                }
                member.emit(ctx)?;
                first = false;
            }
            write!(ctx, ")")?;
        }
        Ok(())
    }
}

impl Emit for Ident {
    fn emit(&self, ctx: &mut EmitCtx<'_>) -> io::Result<()> {
        let Self(name) = self;
        write!(ctx, "{}", name)?;
        Ok(())
    }
}
