pub fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}
