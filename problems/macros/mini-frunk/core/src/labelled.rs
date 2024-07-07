#![forbid(unsafe_code)]
pub trait LabelledGeneric {
    type Repr;
    fn into(self) -> Self::Repr;
    fn from(repr: Self::Repr) -> Self;
}

pub fn from_labelled_generic<Dst, Repr>(repr: Repr) -> Dst
where
    Dst: LabelledGeneric<Repr = Repr>,
{
    Dst::from(repr)
}

pub fn into_labelled_generic<Src, Repr>(src: Src) -> Repr
where
    Src: LabelledGeneric<Repr = Repr>,
{
    src.into()
}
pub fn labelled_convert_from<Src, Dst, Repr>(src: Src) -> Dst
where
    Src: LabelledGeneric<Repr = Repr>,
    Dst: LabelledGeneric<Repr = Repr>,
{
    Dst::from(src.into())
}
