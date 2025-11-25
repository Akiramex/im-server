#![allow(dead_code)]

use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MACHINE_ID_BITS: u64 = 5;
const DATACENTER_ID_BITS: u64 = 5;
const SEQUENCE_BITS: u64 = 12;

const MAX_MACHINE_ID: u64 = (1 << MACHINE_ID_BITS) - 1;
const MAX_DATACENTER_ID: u64 = (1 << DATACENTER_ID_BITS) - 1;
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

// 移位偏移量
const MACHINE_ID_SHIFT: u64 = SEQUENCE_BITS;
const DATACENTER_ID_SHIFT: u64 = SEQUENCE_BITS + MACHINE_ID_BITS;
const TIMESTAMP_SHIFT: u64 = SEQUENCE_BITS + MACHINE_ID_BITS + DATACENTER_ID_BITS;

/// 雪花ID生成器
/// 结构：1位符号位(0) + 41位时间戳 + 5位数据中心ID + 5位机器ID + 12位序列号
pub struct SnowflakeGenerator {
    // 基础配置
    machine_id: u64,
    datacenter_id: u64,

    // 内部状态
    last_timestamp: u64,
    sequence: u64,
}

impl SnowflakeGenerator {
    /// 创建一个新的雪花ID生成器
    pub fn new(machine_id: u64, datacenter_id: u64) -> Self {
        SnowflakeGenerator {
            machine_id,
            datacenter_id,
            last_timestamp: 0,
            sequence: 0,
        }
    }

    pub fn next_id(&mut self) -> u64 {
        let mut current_timestamp = self.current_timestamp();

        // 检查时钟回拨
        if current_timestamp < self.last_timestamp {
            current_timestamp = self.last_timestamp;
        }

        // 如果是同一毫秒，递增序列号
        if current_timestamp == self.last_timestamp {
            self.sequence = (self.sequence + 1) & MAX_SEQUENCE;
            // 序列号溢出，等待下一毫秒
            if self.sequence == 0 {
                self.sequence = self.wait_next_millis(current_timestamp);
            }
        } else {
            // 新的时间戳，重置序列号
            self.sequence = 0;
        }

        self.last_timestamp = current_timestamp;

        // 组合生成ID
        (current_timestamp << TIMESTAMP_SHIFT)
            | (self.datacenter_id << DATACENTER_ID_SHIFT)
            | (self.machine_id << MACHINE_ID_SHIFT)
            | self.sequence
    }

    /// 获取当前时间戳（毫秒）
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }

    /// 等待下一毫秒
    fn wait_next_millis(&self, last_timestamp: u64) -> u64 {
        let mut current_timestamp = self.current_timestamp();
        while current_timestamp <= last_timestamp {
            current_timestamp = self.current_timestamp();
        }
        current_timestamp
    }

    /// 从ID解析出时间戳
    pub fn parse_timestamp(id: u64) -> u64 {
        id >> TIMESTAMP_SHIFT
    }

    /// 从ID解析出数据中心ID
    pub fn parse_datacenter_id(id: u64) -> u64 {
        (id >> DATACENTER_ID_SHIFT) & MAX_DATACENTER_ID
    }

    /// 从ID解析出机器ID
    pub fn parse_machine_id(id: u64) -> u64 {
        (id >> MACHINE_ID_SHIFT) & MAX_MACHINE_ID
    }

    /// 从ID解析出序列号
    pub fn parse_sequence(id: u64) -> u64 {
        id & MAX_SEQUENCE
    }
}

pub static SNOWFLAKE_GENERATOR: LazyLock<Mutex<SnowflakeGenerator>> =
    LazyLock::new(|| Mutex::new(SnowflakeGenerator::new(1, 1)));

pub fn generate_snowflake_id() -> u64 {
    SNOWFLAKE_GENERATOR.lock().unwrap().next_id()
}

pub fn generate_snowflake_id_with_config(machine_id: u64, datacenter_id: u64) -> u64 {
    let mut generator = SnowflakeGenerator::new(machine_id, datacenter_id);
    generator.next_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator() {
        let id = generate_snowflake_id();
        let timestamp = SnowflakeGenerator::parse_timestamp(id);
        let datacenter_id = SnowflakeGenerator::parse_datacenter_id(id);
        let machine_id = SnowflakeGenerator::parse_machine_id(id);
        let sequence = SnowflakeGenerator::parse_sequence(id);
        println!("timestamp : {timestamp}");
        println!("datacenter_id : {datacenter_id}");
        println!("machine_id : {machine_id}");
        println!("sequence : {sequence}");
        println!("snowflake id: {id}");
    }

    #[test]
    fn test_snowflake_mul_thread() {
        let mut handles = Vec::new();
        for _ in 0..10 {
            let handle = std::thread::spawn(|| {
                let id = generate_snowflake_id();
                let timestamp = SnowflakeGenerator::parse_timestamp(id);
                let datacenter_id = SnowflakeGenerator::parse_datacenter_id(id);
                let machine_id = SnowflakeGenerator::parse_machine_id(id);
                let sequence = SnowflakeGenerator::parse_sequence(id);
                println!("timestamp : {timestamp}");
                println!("datacenter_id : {datacenter_id}");
                println!("machine_id : {machine_id}");
                println!("sequence : {sequence}");
                println!("snowflake id: {id}");
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
