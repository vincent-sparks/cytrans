#[macro_export]
macro_rules! rejection_enum {
    ($name:ident, {$($field:ident),*}) => {
        pub enum $name {
            $($field($field)),*
        }

        $(
            impl From<$field> for $name {
                fn from(x: $field) -> Self {
                    $name::$field(x)
                }
            }
        )*

        impl axum::response::IntoResponse for $name {
            fn into_response(self) -> axum::response::Response {
                match self {
                    $($name::$field(e) => e.into_response()),*
                }
            }
        }
    };
}
