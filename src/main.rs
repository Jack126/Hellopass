// 说明：本二进制是无头自测/演示入口；api/database/security 等共享模块主要供
// securepass_tauri 使用。仅编译本二进制时，未被调用的共享项产生 dead_code 警告属预期，
// 故在此 crate 级关闭该 lint（不影响 securepass_tauri 的诊断）。
#![allow(dead_code)]

mod security;
mod database;
mod api;
mod utils;
mod error;

use database::Database;

fn main() {
    println!("SecurePass - 密码管理器");
    println!("========================");
    
    // 初始化日志
    println!("📝 初始化系统...");
    
    // 获取应用目录
    let app_data_dir = std::env::current_dir().unwrap();
    println!("📁 应用目录: {:?}", app_data_dir);
    
    // 初始化数据库
    let database_path = app_data_dir.join("passwords.db");
    let database = match Database::new(&database_path) {
        Ok(db) => {
            println!("✅ 数据库初始化成功");
            db
        }
        Err(e) => {
            println!("❌ 数据库初始化失败: {}", e);
            return;
        }
    };
    
    // 测试加密功能
    println!("\n🔐 测试加密功能...");
    match security::SecureCryptoEngine::new("test_password", None) {
        Ok(engine) => {
            println!("✅ 加密引擎初始化成功");
            
            // 测试加密/解密
            let test_data = b"Hello, World! This is a test password.";
            match engine.encrypt(test_data) {
                Ok(encrypted) => {
                    println!("✅ 数据加密成功 (长度: {})", encrypted.len());
                    match engine.decrypt(&encrypted) {
                        Ok(decrypted) => {
                            if decrypted == test_data {
                                println!("✅ 数据解密成功，数据完整性验证通过");
                            } else {
                                println!("❌ 数据解密失败，数据不匹配");
                            }
                        }
                        Err(e) => println!("❌ 数据解密失败: {}", e),
                    }
                }
                Err(e) => println!("❌ 数据加密失败: {}", e),
            }
        }
        Err(e) => println!("❌ 加密引擎初始化失败: {}", e),
    }
    
    // 测试主密码管理
    println!("\n🔑 测试主密码管理...");
    let mut master_password_manager = security::MasterPasswordManager::new(".");
    
    match master_password_manager.set_master_password("TestPass123!@#") {
        Ok(_) => {
            println!("✅ 主密码设置成功");
            
            // 测试密码验证
            match master_password_manager.verify_password("TestPass123!@#") {
                Ok(_) => println!("✅ 主密码验证成功"),
                Err(e) => println!("❌ 主密码验证失败: {}", e),
            }
            
            // 测试密码强度验证
            let weak_passwords = ["123", "password", "123456"];
            for password in weak_passwords {
                if master_password_manager.validate_password_strength_inner(password) {
                    println!("❌ 密码 '{}' 应该被拒绝但通过了", password);
                } else {
                    println!("✅ 密码 '{}' 正确被拒绝", password);
                }
            }
            
            let strong_password = "SecurePass123!@#Test";
            if master_password_manager.validate_password_strength_inner(strong_password) {
                println!("✅ 强密码 '{}' 正确通过", strong_password);
            } else {
                println!("❌ 强密码 '{}' 被错误拒绝", strong_password);
            }
        }
        Err(e) => println!("❌ 主密码设置失败: {}", e),
    }
    
    // 测试数据库操作
    println!("\n🗄️ 测试数据库操作...");
    match database.get_stats() {
        Ok(stats) => {
            println!("✅ 数据库统计:");
            println!("   - 密码数量: {}", stats.password_count);
            println!("   - 配置数量: {}", stats.config_count);
            println!("   - 审计日志数量: {}", stats.audit_count);
            println!("   - 备份数量: {}", stats.backup_count);
        }
        Err(e) => println!("❌ 获取数据库统计失败: {}", e),
    }
    
    // 测试数据库完整性
    match database.check_integrity() {
        Ok(is_valid) => {
            if is_valid {
                println!("✅ 数据库完整性检查通过");
            } else {
                println!("❌ 数据库完整性检查失败");
            }
        }
        Err(e) => println!("❌ 数据库完整性检查出错: {}", e),
    }
    
    println!("\n🎉 所有测试完成！");
    println!("========================");
    println!("SecurePass 密码管理器核心功能测试完成");
    println!("系统已准备好进行下一步开发");
}