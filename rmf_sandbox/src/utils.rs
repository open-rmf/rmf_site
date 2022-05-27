/// Returns `true` if the option has a value and that value is equal to the default value.
/// Returns `false` otherwise.
pub fn is_option_default<T: Default + PartialEq>(v: &Option<T>) -> bool {
    match v {
        Some(v) => *v == T::default(),
        None => true,
    }
}
