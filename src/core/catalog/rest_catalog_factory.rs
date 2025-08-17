use crate::config::yaml_reader::{GCSProperties, ObjectStorage, S3Properties, Storage};
use iceberg_catalog_rest::{RestCatalog, RestCatalogConfig};
use std::collections::HashMap;

pub async fn create_rest_catalog(storage: Storage) -> RestCatalog {
    let properties = match storage.object_storage {
        ObjectStorage::S3(properties) => s3_props(properties),
        ObjectStorage::GCS(properties) => gcs_props(properties),
    };

    let config = build_catalog_config(properties, storage.warehouse);
    RestCatalog::new(config)
}

fn s3_props(properties: S3Properties) -> HashMap<String, String> {
    HashMap::from([
        ("aws.region".to_string(), properties.aws_region),
        ("aws.endpoint".to_string(), properties.aws_endpoint),
        (
            "aws.access_key_id".to_string(),
            properties.aws_access_key_id,
        ),
        (
            "aws.secret_access_key".to_string(),
            properties.aws_secret_access_key,
        ),
        (
            "path-style-access".to_string(),
            properties.path_style_access.to_string(),
        ),
    ])
}

fn gcs_props(_properties: GCSProperties) -> HashMap<String, String> {
    // TODO: implement when GCS is needed
    HashMap::new()
}

fn build_catalog_config(
    properties: HashMap<String, String>,
    warehouse_name: String,
) -> RestCatalogConfig {
    let endpoint = properties
        .get("aws.endpoint")
        .expect("Missing endpoint property")
        .clone();

    RestCatalogConfig::builder()
        .uri(endpoint)
        .warehouse(warehouse_name)
        .props(properties)
        .build()
}

#[cfg(test)]
pub mod test {
    use crate::config::yaml_reader::{ObjectStorage, S3Properties, Storage};
    use crate::core::catalog::rest_catalog_factory::{create_rest_catalog, s3_props};

    #[test]
    fn test_s3_props_mapping() {
        let props = S3Properties {
            aws_region: "us-east-1".to_string(),
            aws_endpoint: "http://localhost:9000".to_string(),
            aws_access_key_id: "minio".to_string(),
            aws_secret_access_key: "secret".to_string(),
            path_style_access: true,
        };

        let map = s3_props(props);

        assert_eq!(map.get("aws.region").unwrap(), "us-east-1");
        assert_eq!(map.get("aws.endpoint").unwrap(), "http://localhost:9000");
        assert_eq!(map.get("aws.access_key_id").unwrap(), "minio");
        assert_eq!(map.get("aws.secret_access_key").unwrap(), "secret");
        assert_eq!(map.get("path-style-access").unwrap(), "true");
    }

    #[tokio::test]
    async fn test_fetch_catalog_s3() {
        let storage = Storage {
            warehouse: "log".to_string(),
            namespace: "tests".to_string(),
            object_storage: ObjectStorage::S3(S3Properties {
                aws_region: "us-east-1".to_string(),
                aws_endpoint: "http://localhost:9000".to_string(),
                aws_access_key_id: "minio".to_string(),
                aws_secret_access_key: "secret".to_string(),
                path_style_access: true,
            }),
        };

        let _catalog = create_rest_catalog(storage).await;
        // You can add further assertions if needed.
    }
}
