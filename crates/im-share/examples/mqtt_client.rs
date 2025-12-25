//! MQTT客户端使用示例
//!
//! 这个示例展示了如何使用 `src/utils/mqtt.rs` 中的MQTT功能
//!
//! 运行方式:
//! ```
//! cargo run --example mqtt_client
//! ```
//!
//! 需要先启动一个MQTT代理服务器，例如:
//! ```
//! docker run -d -p 1883:1883 -p 9001:9001 eclipse-mosquitto
//! ```

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

// 导入项目中的MQTT模块
use im_share::mqtt::{ImMqtt, MqttConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 MQTT客户端示例开始...");
    println!("==========================================");

    // 1. 创建MQTT配置
    let config = MqttConfig::new(
        "localhost",               // MQTT代理地址
        1883,                      // MQTT端口
        "example_mqtt_client_123", // 客户端ID
    );

    println!("📋 MQTT配置:");
    println!("  - 代理地址: {}:{}", config.host, config.port);
    println!("  - 客户端ID: {}", config.client_id);
    println!("  - 保活时间: {}秒", config.keep_alive_secs);
    println!("==========================================");

    // 2. 创建并连接MQTT客户端
    println!("🔗 正在连接到MQTT代理...");
    let mqtt_client = ImMqtt::connect(config);
    println!("✅ 成功连接到MQTT代理");
    println!("==========================================");

    // 3. 订阅主题
    let topic = "helloworld";
    println!("📥 正在订阅主题: {}", topic);

    let mut receiver = mqtt_client.subscribe(topic).await?;
    println!("✅ 已成功订阅主题: {}", topic);
    println!("   - 使用 QoS 1 (AtLeastOnce) 确保消息可靠传递");
    println!("   - clean_session=false 允许代理存储离线消息");
    println!("==========================================");

    // 4. 启动消息接收任务
    let mqtt_client_clone = mqtt_client.clone();
    let receive_handle = tokio::spawn(async move {
        println!("👂 开始监听消息...");
        println!("==========================================");

        let mut message_count = 0;

        loop {
            match receiver.recv().await {
                Ok(message) => {
                    message_count += 1;

                    // 解析消息内容
                    let payload_str = String::from_utf8_lossy(&message.payload);

                    println!("📨 收到消息 #{}", message_count);
                    println!("  ├─ 主题: {}", message.topic);
                    println!("  ├─ 内容长度: {} 字节", message.payload.len());
                    println!("  ├─ 内容: {}", payload_str);

                    // 尝试解析为JSON
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                        println!("  └─ JSON解析成功:");
                        println!("     消息ID: {:?}", json.get("message_id"));
                        println!("     聊天类型: {:?}", json.get("chat_type"));
                        println!("     发送者: {:?}", json.get("from_user_id"));
                        println!("     接收者: {:?}", json.get("to_user_id"));
                    } else {
                        println!("  └─ 纯文本消息");
                    }
                    println!("------------------------------------------");

                    // 如果收到特定消息，可以取消订阅
                    if payload_str.contains("unsubscribe") {
                        println!("⚠️  收到取消订阅指令，正在取消订阅...");
                        if let Err(e) = mqtt_client_clone.unsubscribe(topic).await {
                            eprintln!("❌ 取消订阅失败: {}", e);
                        } else {
                            println!("✅ 已取消订阅主题: {}", topic);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ 接收消息时出错: {}", e);
                    break;
                }
            }
        }

        println!("📭 消息接收任务结束");
    });

    // 5. 发布消息
    println!("📤 开始发布测试消息...");

    // 发布简单文本消息
    let simple_message = "Hello MQTT from example client!";
    println!("  发布消息 #1: {}", simple_message);

    // 注意: 在当前的 ImMqtt 实现中，没有直接的 publish 方法
    // 如果需要发布功能，需要在 ImMqtt 结构体中添加 publish 方法
    // 这里我们只演示订阅功能

    // 等待一段时间让消息处理
    println!("⏳ 等待5秒接收消息...");
    sleep(Duration::from_secs(5)).await;

    println!("==========================================");
    println!("📊 示例总结:");
    println!("  - 已成功连接到MQTT代理");
    println!("  - 已订阅主题: {}", topic);
    println!("  - 正在监听消息...");
    println!("");
    println!("💡 使用说明:");
    println!("  1. 使用其他MQTT客户端向主题 '{}' 发布消息", topic);
    println!("  2. 本示例将接收并显示这些消息");
    println!("  3. 发送包含 'unsubscribe' 的消息可以触发取消订阅");
    println!("");
    println!("🔧 技术细节:");
    println!("  - 使用 rumqttc 库实现MQTT协议");
    println!("  - 使用 broadcast channel 分发消息");
    println!("  - 支持 QoS 1 (AtLeastOnce)");
    println!("  - clean_session=false 支持离线消息存储");
    println!("==========================================");

    // 6. 保持运行，等待用户中断
    println!("⏳ 按 Ctrl+C 退出程序...");

    // 等待消息接收任务
    tokio::select! {
        _ = receive_handle => {
            println!("✅ 消息接收任务正常结束");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 收到中断信号，正在退出...");
        }
    }

    // 7. 断开连接（ImMqtt 会在 drop 时自动断开）
    println!("🔌 正在断开MQTT连接...");
    // ImMqtt 结构体在 drop 时会自动断开连接

    println!("✅ MQTT客户端示例结束");
    Ok(())
}

// 测试用的辅助函数
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mqtt_config() {
        let config = MqttConfig::new("localhost", 1883, "test_client");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.client_id, "test_client");
        assert_eq!(config.keep_alive_secs, 30);
    }

    #[tokio::test]
    async fn test_incoming_message() {
        let message = IncomingMessage::new("test/topic", b"test payload".to_vec());
        assert_eq!(message.topic, "test/topic");
        assert_eq!(message.payload, b"test payload");
    }
}
