#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionView {
    Notes,
    Pinned,
    Today,
    Todos,
    Untagged,
    Archive,
    Trash,
}
