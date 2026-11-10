// 说明：本二进制是无头自测/演示入口；api/database/security 等共享模块主要供
// securepass 使用。仅编译本二进制时，未被调用的共享项产生 dead_code 警告属预期，
// 故在此 crate 级关闭该 lint（不影响 securepass 的诊断）。
#![allow(dead_code)]

mod security;
mod database;
mod api;
mod utils;
mod error;

use std::path::PathBuf;

use api::dto::NewPassword;
use api::runtime::VaultRuntime;
use database::Database;
use rand::Rng;

/// 演示数据：生成 count 条随机记录到一个全新的库（会创建主密码配置）。
/// 用法：securepass_cli seed-demo <db> <config_dir> <master_password> <count>
fn seed_demo(args: &[String]) {
    if args.len() < 6 {
        eprintln!("用法: securepass_cli seed-demo <db> <config_dir> <master_password> <count>");
        std::process::exit(2);
    }
    let db = PathBuf::from(&args[2]);
    let cfg = PathBuf::from(&args[3]);
    let password = &args[4];
    let count: usize = args[5].parse().expect("count 必须是数字");
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::create_dir_all(&cfg).ok();

    let mut rt = VaultRuntime::new(cfg, db);
    let setup = rt.setup(password);
    if !setup.success {
        eprintln!("初始化失败: {:?}", setup.error);
        std::process::exit(1);
    }

    let sites: &[(&str, &str)] = &[
        ("GitHub", "https://github.com"),
        ("GitLab", "https://gitlab.com"),
        ("Google", "https://accounts.google.com"),
        ("Microsoft", "https://login.microsoftonline.com"),
        ("Apple ID", "https://appleid.apple.com"),
        ("Steam", "https://store.steampowered.com"),
        ("淘宝", "https://www.taobao.com"),
        ("京东", "https://www.jd.com"),
        ("支付宝", "https://www.alipay.com"),
        ("微信", "https://wx.qq.com"),
        ("QQ", "https://im.qq.com"),
        ("微博", "https://weibo.com"),
        ("知乎", "https://www.zhihu.com"),
        ("豆瓣", "https://www.douban.com"),
        ("B站", "https://www.bilibili.com"),
        ("网易云音乐", "https://music.163.com"),
        ("阿里云", "https://www.aliyun.com"),
        ("腾讯云", "https://cloud.tencent.com"),
        ("Cloudflare", "https://dash.cloudflare.com"),
        ("Namecheap", "https://www.namecheap.com"),
        ("Vercel", "https://vercel.com"),
        ("Figma", "https://www.figma.com"),
        ("Notion", "https://www.notion.so"),
        ("Slack", "https://slack.com"),
        ("Zoom", "https://zoom.us"),
        ("PayPal", "https://www.paypal.com"),
        ("Amazon", "https://www.amazon.com"),
        ("Netflix", "https://www.netflix.com"),
        ("Spotify", "https://www.spotify.com"),
        ("Dropbox", "https://www.dropbox.com"),
    ];
    let tag_pool = ["工作", "个人", "金融", "社交", "开发", "购物", "娱乐"];
    let charset: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();
    let mut ok_count = 0usize;

    for i in 0..count {
        let (site, url) = sites[i % sites.len()];
        let pwd: String = (0..18)
            .map(|_| charset[rng.gen_range(0..charset.len())])
            .collect();
        let mut tags = vec![tag_pool[rng.gen_range(0..tag_pool.len())].to_string()];
        if i % 4 == 0 {
            tags.push("演示".to_string());
        }
        let notes = if i % 5 == 0 {
            Some(format!("演示记录 #{}", i + 1))
        } else {
            None
        };
        let res = rt.add_password(NewPassword {
            site_name: format!("{} {}", site, if i >= sites.len() { i / sites.len() + 1 } else { 1 }),
            site_url: url.to_string(),
            username: format!("user{:03}@example.com", i + 1),
            password: pwd,
            notes,
            tags: Some(tags),
            priority: Some((i % 4) as i32),
        });
        if res.success {
            ok_count += 1;
        } else {
            eprintln!("第 {} 条失败: {:?}", i + 1, res.error);
        }
    }

    println!("已生成 {} / {} 条演示记录", ok_count, count);
    println!("主密码: {}", password);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("seed-demo") {
        seed_demo(&args);
        return;
    }

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