#![forbid(unsafe_code)]

pub trait Generic {
    type Repr;
    fn into(self) -> Self::Repr;
    fn from(repr: Self::Repr) -> Self;
}

pub fn from_generic<Dst, Repr>(repr: Repr) -> Dst
where
    Dst: Generic<Repr = Repr>,
{
    Dst::from(repr)
}

pub fn into_generic<Src, Repr>(src: Src) -> Repr
where
    Src: Generic<Repr = Repr>,
{
    src.into()
}
pub fn convert_from<Src, Dst, Repr>(src: Src) -> Dst
where
    Src: Generic<Repr = Repr>,
    Dst: Generic<Repr = Repr>,
{
    Dst::from(src.into())
}
