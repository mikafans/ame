use crate::enums::{ImageSourceType, SourceType};

trait ImageFetcher {
    type ImageType: ImageSourceType;


}