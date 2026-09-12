pub trait Compare<T>
where
    T: PartialEq + PartialOrd,
{
    fn key(&self) -> T;
}
