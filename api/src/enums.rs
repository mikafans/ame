#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SourceType {
    Yandere,
    Konachan,
}

pub enum ViewType {
    Default,
    Popular,
    
}

pub trait ImageSourceType {}

impl ImageSourceType for SourceType {}