#![forbid(unsafe_code)]

pub mod symbols {
    macro_rules! generate_char_enums {
        ($($char: ident)*) => {

            $(
                #[allow(non_snake_case, non_camel_case_types)]
                #[derive(Debug, PartialEq, Eq)]
                pub enum $char {}
            )*
        };
    }
    generate_char_enums!(
        a b c d e f g h i j k l m n o p q r s t u v w x y z
        A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
        __
    );
}

////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, PartialEq, Eq)]
pub struct Field<N, T> {
    pub name_type_holder: std::marker::PhantomData<N>,
    pub value: T,
}

#[macro_export]
macro_rules! field {
    (($($name: ty),*), $val: expr) => {{
        ::mini_frunk_core::field::Field::< ($($name),+),_> {
            name_type_holder: std::marker::PhantomData,
            value: $val,
        }
    }};
}
