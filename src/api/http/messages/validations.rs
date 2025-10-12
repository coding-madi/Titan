use validator::ValidationError;

pub fn validate_group_by(_group_by: &Vec<String>) -> Result<(), ValidationError> {
    Ok(())
}

pub fn validate_flight_name(_flight_name: &str) -> Result<(), ValidationError> {
    Ok(())
}

pub fn validate_script(_script: &str) -> Result<(), ValidationError> {
    Ok(())
}
