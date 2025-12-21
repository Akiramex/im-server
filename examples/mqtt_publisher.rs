//! MQTT发布者示例
//!
//! 这个示例展示了如何使用MQTT客户端只发布消息，不订阅任何主题
//! 消息内容从命令行用户输入，主题只输入一次
//!
//! 运行方式:
//! ```
//! cargo run --example mqtt_publisher
//! ```
//!
//! 需要先启动一个MQTT代理服务器，例如:
//! ```
//! docker run -d -p 1883:1883 -p 9001:9001 eclipse-mosquitto
//! ```

use anyhow::Result;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};

// 导入项目中的MQTT模块
use im_server::utils::mqtt::{ImMqtt, MqttConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 MQTT发布者示例开始...");
    println!("==========================================");

    // 1. 获取MQTT代理地址
    println!("1. MQTT代理地址配置 ！⚠️ 填写 localhost 会报错");
    print!("   请输入MQTT代理地址 (默认: broker.emqx.io): ");
    io::stdout().flush()?;
    let mut host = String::new();
    io::stdin().read_line(&mut host)?;
    let host = host.trim();
    let host = if host.is_empty() {
        "broker.emqx.io"
    } else {
        host
    };
    println!("   ✅ 代理地址: {}", host);

    // 2. 获取MQTT端口
    println!("\n2. MQTT端口配置");
    print!("   请输入MQTT端口 (默认: 1883): ");
    io::stdout().flush()?;
    let mut port_input = String::new();
    io::stdin().read_line(&mut port_input)?;
    let port_input = port_input.trim();
    let port: u16 = if port_input.is_empty() {
        1883
    } else {
        match port_input.parse() {
            Ok(p) => p,
            Err(_) => {
                println!("   ⚠️  端口号无效，使用默认值: 1883");
                1883
            }
        }
    };
    println!("   ✅ 端口: {}", port);

    // 3. 获取客户端ID
    println!("\n3. 客户端ID配置");
    print!("   请输入客户端ID (默认: mqtt_publisher): ");
    io::stdout().flush()?;
    let mut client_id = String::new();
    io::stdin().read_line(&mut client_id)?;
    let client_id = client_id.trim();
    let client_id = if client_id.is_empty() {
        "mqtt_publisher".to_string()
    } else {
        client_id.to_string()
    };
    println!("   ✅ 客户端ID: {}", client_id);

    // 4. 获取发布主题（只输入一次）
    println!("\n4. 发布主题配置");
    println!("   📝 请输入发布主题（只输入一次，后续所有消息都将发布到此主题）");
    print!("   主题: ");
    io::stdout().flush()?;
    let mut topic = String::new();
    io::stdin().read_line(&mut topic)?;
    let topic = topic.trim().to_string();

    if topic.is_empty() {
        println!("   ❌ 错误: 主题不能为空！");
        return Ok(());
    }
    println!("   ✅ 发布主题: {}", topic);

    println!("==========================================");
    println!("📋 MQTT配置:");
    println!("  - 代理地址: {}:{}", host, port);
    println!("  - 客户端ID: {}", client_id);
    println!("  - 发布主题: {}", topic);
    println!("==========================================");

    // 5. 创建MQTT配置并连接
    println!("🔗 正在连接到MQTT代理...");
    let config = MqttConfig::new(host, port, &client_id);
    let mqtt_client = ImMqtt::connect(config);
    println!("✅ 成功连接到MQTT代理");
    println!("==========================================");

    println!("\n==========================================");
    println!("🎯 配置完成！发布设置:");
    println!("  - 代理地址: {}:{}", host, port);
    println!("  - 客户端ID: {}", client_id);
    println!("  - 发布主题: {}", topic);
    println!("==========================================");

    // 8. 开始消息发布循环
    println!("\n📤 开始发布消息...");
    println!("  输入消息内容并按回车发送");
    println!("  输入 'quit' 或 'exit' 退出程序");
    println!("  输入 'clear' 清空屏幕");
    println!("  输入 'help' 显示帮助");
    println!("==========================================");

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    let mut message_count = 0;

    loop {
        print!("消息 {} > ", message_count + 1);
        io::stdout().flush()?;

        if let Some(line) = lines.next_line().await? {
            let message = line.trim();

            // 检查退出命令
            if message.eq_ignore_ascii_case("quit") || message.eq_ignore_ascii_case("exit") {
                println!("🛑 退出程序...");
                break;
            }

            // 检查清屏命令
            if message.eq_ignore_ascii_case("clear") {
                print!("{}[2J", 27 as char); // 清屏
                print!("{}[H", 27 as char); // 光标回到左上角
                io::stdout().flush()?;
                println!("屏幕已清空");
                println!("==========================================");
                continue;
            }

            // 检查帮助命令
            if message.eq_ignore_ascii_case("help") {
                println!("\n📖 帮助信息:");
                println!("  - 输入消息内容: 发布消息到主题 '{}'", topic);
                println!("  - quit/exit: 退出程序");
                println!("  - clear: 清空屏幕");
                println!("  - help: 显示此帮助信息");
                println!("  - 空行: 跳过不发布");
                println!("------------------------------------------");
                continue;
            }

            // 跳过空消息
            if message.is_empty() {
                continue;
            }

            // 发布消息
            message_count += 1;
            println!("  📤 正在发布消息 #{}...", message_count);

            match mqtt_client
                .publish(&topic, message.as_bytes().to_vec())
                .await
            {
                Ok(_) => {
                    println!("  ✅ 消息 #{} 发布成功", message_count);
                    println!("     ├─ 主题: {}", topic);
                    println!("     ├─ 内容: {}", message);
                    println!("     ├─ 长度: {} 字节", message.len());
                }
                Err(e) => {
                    println!("  ❌ 消息 #{} 发布失败: {}", message_count, e);
                    println!("     错误详情: {}", e);
                }
            }

            println!("  ------------------------------------------");
        } else {
            // EOF (Ctrl+D on Unix, Ctrl+Z on Windows)
            println!("\n📭 输入结束，退出程序...");
            break;
        }
    }

    // 9. 统计和总结
    println!("\n==========================================");
    println!("📊 发布统计:");
    println!("  ├─ 总发布消息数: {}", message_count);
    println!("  ├─ 发布主题: {}", topic);
    println!("==========================================");

    // 10. 断开连接
    println!("🔌 正在断开MQTT连接...");
    // ImMqtt 结构体在 drop 时会自动断开连接
    drop(mqtt_client);

    println!("✅ MQTT发布者示例结束");
    Ok(())
}

// 测试用的辅助函数
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_parsing() {
        assert_eq!(QoS::AtMostOnce as u8, 0);
        assert_eq!(QoS::AtLeastOnce as u8, 1);
        assert_eq!(QoS::ExactlyOnce as u8, 2);
    }

    #[test]
    fn test_message_validation() {
        // 空消息应该被跳过
        assert!("".is_empty());

        // 退出命令
        assert!("quit".eq_ignore_ascii_case("QUIT"));
        assert!("exit".eq_ignore_ascii_case("EXIT"));

        // 清屏命令
        assert!("clear".eq_ignore_ascii_case("CLEAR"));
    }
}
