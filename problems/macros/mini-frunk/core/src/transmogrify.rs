#![forbid(unsafe_code)]
use crate::hlist::{HCons, HNil};
use crate::labelled::LabelledGeneric;

pub enum Here {}
pub struct There<T>(std::marker::PhantomData<T>);

pub trait Plucker<Target, Indices> {
    type Remainder;
    fn pluck(self) -> (Target, Self::Remainder);
}

impl<Target, Tail> Plucker<Target, Here> for HCons<Target, Tail> {
    type Remainder = Tail;

    fn pluck(self) -> (Target, Self::Remainder) {
        (self.head, self.tail)
    }
}

impl<Head, Tail, Target, TailIndices> Plucker<Target, There<TailIndices>> for HCons<Head, Tail>
where
    Tail: Plucker<Target, TailIndices>,
{
    type Remainder = HCons<Head, <Tail as Plucker<Target, TailIndices>>::Remainder>;

    fn pluck(self) -> (Target, Self::Remainder) {
        let (target, tail_reminder) = self.tail.pluck();
        (
            target,
            HCons {
                head: self.head,
                tail: tail_reminder,
            },
        )
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Sculptor<Target, Indices> {
    type Remainder;
    fn sculpt(self) -> (Target, Self::Remainder);
}

impl<Sorce> Sculptor<HNil, Here> for Sorce {
    type Remainder = Sorce;

    fn sculpt(self) -> (HNil, Self::Remainder) {
        (HNil, self)
    }
}

impl<THead, TTail, SHead, STail, IndexHead, IndexTail>
    Sculptor<HCons<THead, TTail>, HCons<IndexHead, IndexTail>> for HCons<SHead, STail>
where
    HCons<SHead, STail>: Plucker<THead, IndexHead>,
    <HCons<SHead, STail> as Plucker<THead, IndexHead>>::Remainder: Sculptor<TTail, IndexTail>,
{
    type Remainder = <<HCons<SHead, STail> as Plucker<THead, IndexHead>>::Remainder as Sculptor<
        TTail,
        IndexTail,
    >>::Remainder;

    fn sculpt(self) -> (HCons<THead, TTail>, Self::Remainder) {
        let (target_head, source_reminder) = self.pluck();
        let (target_tail, tail_reminder) = source_reminder.sculpt();
        (
            HCons {
                head: target_head,
                tail: target_tail,
            },
            tail_reminder,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Transmogrifier<Dst, Indices> {
    fn transmogrify(self) -> Dst;
}

impl<Dst, Indices, Src> Transmogrifier<Dst, Indices> for Src
where
    Src: LabelledGeneric,
    Dst: LabelledGeneric,
    <Src as LabelledGeneric>::Repr: Sculptor<<Dst as LabelledGeneric>::Repr, Indices>,
{
    fn transmogrify(self) -> Dst {
        transmogrify_from(self)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn transmogrify_from<Src, Dst, Indices>(src: Src) -> Dst
where
    Src: LabelledGeneric,
    Dst: LabelledGeneric,
    <Src as LabelledGeneric>::Repr: Sculptor<<Dst as LabelledGeneric>::Repr, Indices>,
{
    let src_repr = src.into();
    let (dst_repr, _reminder) = src_repr.sculpt();
    <Dst as LabelledGeneric>::from(dst_repr)
}
