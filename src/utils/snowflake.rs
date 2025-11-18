use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// 雪花ID生成器
/// 结构：1位符号位(0) + 41位时间戳 + 5位数据中心ID + 5位机器ID + 12位序列号
pub struct Snowflake {
    // 基础配置
    machine_id: u64,
    datacenter_id: u64,

    // 内部状态
    last_timestamp: AtomicU64,
    sequence: AtomicU64,
}

impl Snowflake {
    // 常量定义
    const MACHINE_ID_BITS: u64 = 5;
    const DATACENTER_ID_BITS: u64 = 5;
    const SEQUENCE_BITS: u64 = 12;

    const MAX_MACHINE_ID: u64 = (1 << Self::MACHINE_ID_BITS) - 1;
    const MAX_DATACENTER_ID: u64 = (1 << Self::DATACENTER_ID_BITS) - 1;
    const MAX_SEQUENCE: u64 = (1 << Self::SEQUENCE_BITS) - 1;

    // 移位偏移量
    const MACHINE_ID_SHIFT: u64 = Self::SEQUENCE_BITS;
    const DATACENTER_ID_SHIFT: u64 = Self::SEQUENCE_BITS + Self::MACHINE_ID_BITS;
    const TIMESTAMP_SHIFT: u64 =
        Self::SEQUENCE_BITS + Self::MACHINE_ID_BITS + Self::DATACENTER_ID_BITS;

    /// 创建一个新的雪花ID生成器
    pub fn new(machine_id: u64, datacenter_id: u64) -> Self {
        Snowflake {
            machine_id,
            datacenter_id,
            last_timestamp: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut current_sequence = self.sequence.load(Ordering::Relaxed);
        let mut last_timestamp = self.last_timestamp.load(Ordering::Relaxed);
        let mut current_timestamp = self.current_timestamp();

        // 检查时钟回拨
        if current_timestamp < last_timestamp {
            current_timestamp = last_timestamp;
        }

        // 如果是同一毫秒，递增序列号
        if current_timestamp == last_timestamp {
            current_sequence = (current_sequence + 1) & Self::MAX_SEQUENCE;
            // 序列号溢出，等待下一毫秒
            if current_sequence == 0 {
                current_sequence = self.wait_next_millis(current_timestamp);
            }
        } else {
            // 新的时间戳，重置序列号
            current_sequence = 0;
        }

        // 尝试原子更新状态
        loop {
            match self.last_timestamp.compare_exchange_weak(
                last_timestamp,
                current_timestamp,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.sequence.store(current_sequence, Ordering::Relaxed);
                    break;
                }
                Err(updated_last_timestamp) => {
                    current_timestamp = updated_last_timestamp;
                    current_sequence = self.wait_next_millis(current_timestamp);
                }
            }
        }

        // 组合生成ID
        return (last_timestamp << Self::TIMESTAMP_SHIFT)
            | (self.datacenter_id << Self::DATACENTER_ID_SHIFT)
            | (self.machine_id << Self::MACHINE_ID_SHIFT)
            | current_sequence;
    }

    /// 获取当前时间戳（毫秒）
    pub fn current_timestamp(&self) -> u64 {
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
    pub fn parse_timestamp(&self, id: u64) -> u64 {
        id >> Self::TIMESTAMP_SHIFT
    }

    /// 从ID解析出数据中心ID
    pub fn parse_datacenter_id(&self, id: u64) -> u64 {
        (id >> Self::DATACENTER_ID_SHIFT) & Self::MAX_DATACENTER_ID
    }

    /// 从ID解析出机器ID
    pub fn parse_machine_id(&self, id: u64) -> u64 {
        (id >> Self::MACHINE_ID_SHIFT) & Self::MAX_MACHINE_ID
    }

    /// 从ID解析出序列号
    pub fn parse_sequence(&self, id: u64) -> u64 {
        id & Self::MAX_SEQUENCE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator() {
        let generator = Snowflake::new(1, 1);
        let current_timestamp = generator.current_timestamp();
        let id = generator.next_id();

        println!("current_timestamp : {current_timestamp}");
        println!("snowflake id: {id}");
    }
}
