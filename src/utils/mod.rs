use std::sync::LazyLock;

mod snowflake;
use snowflake::Snowflake;
pub static SNOWFLAKE_GENERATOR: LazyLock<Snowflake> = LazyLock::new(|| Snowflake::new(1, 1));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snowflake_generator() {
        let snowflake = SNOWFLAKE_GENERATOR.next_id();
        assert_eq!(SNOWFLAKE_GENERATOR.parse_datacenter_id(snowflake), 1);
        assert_eq!(SNOWFLAKE_GENERATOR.parse_machine_id(snowflake), 1);

        println!("snowflake id: {snowflake}")
    }
}
