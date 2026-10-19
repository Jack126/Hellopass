use std::path::Path;

fn main() {
    println!("SecurePass - 密码管理器");
    println!("========================");
    
    // 初始化数据库
    let database_path = "passwords.db";
    let database = match crate::database::Database::new(database_path) {
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
    match crate::security::SecureCryptoEngine::new("test_password", None) {
        Ok(engine) => {
            println!("✅ 加密引擎初始化成功");
            
            // 测试加密/解密
            let test_data = b"Hello, World!";
            match engine.encrypt(test_data) {
                Ok(encrypted) => {
                    println!("✅ 数据加密成功");
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
    let mut master_password_manager = crate::security::MasterPasswordManager::new(".");
    
    match master_password_manager.set_master_password("TestPass123!@#") {
        Ok(_) => {
            println!("✅ 主密码设置成功");
            
            // 测试密码验证
            match master_password_manager.verify_password("TestPass123!@#") {
                Ok(_) => println!("✅ 主密码验证成功"),
                Err(e) => println!("❌ 主密码验证失败: {}", e),
            }
        }
        Err(e) => println!("❌ 主密码设置失败: {}", e),
    }
    
    println!("========================");
    println!("程序测试完成");
}