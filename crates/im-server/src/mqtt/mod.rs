use im_share::mqtt::{ImMqtt, MqttConfig};
use std::sync::OnceLock;

pub static MQTT_PUBLISHER: OnceLock<MqttPublisher> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct MqttPublisher(ImMqtt);

impl MqttPublisher {
    /// 创建新的 MQTT 发布者（异步）
    ///
    /// # 参数
    /// - `config`: MQTT 配置
    ///
    pub async fn new(config: MqttConfig) -> Self {
        let im = ImMqtt::connect(config);

        // 等待一小段时间让连接有机会建立
        // 注意：这里不能等待连接完全建立，因为 ImMqtt::connect 是同步的
        // 实际的连接建立是在后台异步任务中进行的
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Self(im)
    }

    /// 发布消息到指定主题
    ///
    /// # 参数
    /// - `topic`: 主题名称
    /// - `payload`: 消息负载
    ///
    /// # 返回值
    /// - `Ok(())`: 发布成功
    /// - `Err(anyhow::Error)`: 发布失败的错误信息
    pub async fn publish(&self, topic: &str, payload: Vec<u8>) -> anyhow::Result<()> {
        self.0.publish(topic, payload).await
    }
}

/// 异步初始化 MQTT 客户端
///
/// # 参数
/// - `config`: MQTT 配置
///
/// # 返回值
/// - `Ok(())`: 初始化成功
/// - `Err(anyhow::Error)`: 初始化失败的错误信息
///
/// # 注意
/// - 这个函数只能调用一次，重复调用会返回错误
/// - 必须在 Tokio 运行时中调用
pub async fn init_mqtt_client(config: &MqttConfig) -> anyhow::Result<()> {
    let publisher = MqttPublisher::new(config.clone()).await;

    MQTT_PUBLISHER
        .set(publisher)
        .map_err(|_| anyhow::anyhow!("MQTT client already initialized"))?;

    Ok(())
}

/// 获取 MQTT 发布者实例
///
/// # 返回值
/// - `Ok(MqttPublisher)`: MQTT 发布者实例
/// - `Err(anyhow::Error)`: MQTT 客户端未初始化的错误信息
///
/// # 注意
/// - 必须先调用 `init_mqtt_client` 初始化
pub fn get_mqtt_publisher() -> &'static MqttPublisher {
    MQTT_PUBLISHER
        .get()
        .expect("MQTT client not initialized. Call `init_mqtt_client` first.")
}

/// 检查 MQTT 客户端是否已初始化
///
/// # 返回值
/// - `true`: 已初始化
/// - `false`: 未初始化
pub fn is_mqtt_initialized() -> bool {
    MQTT_PUBLISHER.get().is_some()
}
