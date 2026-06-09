mod asar;
mod fuses;
mod hook;

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::Deserialize;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

const FIXED_REG_CODE: &str = "DreamNya2026";
const FIXED_DATE: &str = "1/1/2099";
const FIXED_IDATE: &str = "12/1/2025";

#[derive(Deserialize, Debug)]
struct MachineCode {
    #[serde(default)]
    v: String,
    #[serde(default)]
    i: String,
    #[serde(default)]
    l: String,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[ERROR] {e}");
    }
    print!("\n按回车键退出...");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
}

fn run() -> Result<(), String> {
    println!("==== Typora Minimal Fix Patch (Rust) ====\n");

    let install_path = find_or_ask_install_path()?;
    let resources = install_path.join("resources");
    let asar_path = resources.join("app.asar");
    let app_dir = resources.join("app");
    let app_bak_dir = resources.join("app.bak");
    let typora_exe = install_path.join("Typora.exe");
    let launch_dist_js = app_dir.join("launch.dist.js");

    if !typora_exe.exists() {
        return Err(format!("未找到 Typora.exe: {}", typora_exe.display()));
    }

    println!("请输入 Typora 离线激活机器码 (machine code):");
    let machine_code = read_line()?.trim().to_string();
    if machine_code.is_empty() {
        return Err("机器码不能为空".into());
    }

    println!("请输入邮箱 (可选, 直接回车使用默认):");
    let mut email = read_line()?.trim().to_string();
    if email.is_empty() {
        email = "typora@china.cn".into();
    }

    let decoded = B64
        .decode(machine_code.as_bytes())
        .map_err(|e| format!("机器码 base64 解码失败: {e}"))?;
    let mc: MachineCode = serde_json::from_slice(&decoded)
        .map_err(|e| format!("机器码 JSON 解析失败: {e}"))?;
    println!("\n  Version    : {}", mc.v);
    println!("  Fingerprint: {}", mc.i);
    println!("  DeviceId   : {}\n", mc.l);

    println!("[*] 正在关闭 Typora 进程...");
    let _ = Command::new("taskkill").args(["/F", "/IM", "Typora.exe"]).output();

    // 需要等待进程彻底结束
    println!("\n[*] 准备开始打补丁流程\n");
    thread::sleep(Duration::from_secs(5));

    if !app_dir.exists() {
        println!("[1/5] 解包 app.asar -> app/ ...");
        if !asar_path.exists() {
            return Err(format!("找不到 {}", asar_path.display()));
        }
        asar::extract_all(&asar_path, &app_dir).map_err(|e| format!("ASAR 解包失败: {e}"))?;
        println!("      完成");
    } else {
        println!("[1/5] 跳过 (app/ 已存在)");
    }

    println!("[2/5] 备份文件...");
    if !app_bak_dir.exists() {
        asar::copy_dir_all(&app_dir, &app_bak_dir).map_err(|e| format!("复制 app.bak 失败: {e}"))?;
    }
    if asar_path.exists() {
        let _ = fs::remove_file(&asar_path);
    }
    let typora_exe_bak = with_extra_suffix(&typora_exe, ".bak");
    if !typora_exe_bak.exists() {
        fs::copy(&typora_exe, &typora_exe_bak).map_err(|e| format!("备份 Typora.exe 失败: {e}"))?;
    }
    println!("      完成");

    println!("[3/5] 修改 Electron Fuses (OnlyLoadAppFromAsar=DISABLE)...");
    match fuses::disable_only_load_app_from_asar(&typora_exe) {
        Ok(true) => println!("      完成"),
        Ok(false) => println!("      [!] 未找到 fuse sentinel 或版本不匹配, 跳过 (可能已是禁用状态)"),
        Err(e) => println!("      [!] 失败: {e}"),
    }

    println!("[4/5] 注入激活代码到 launch.dist.js ...");
    inject_hook(&launch_dist_js, &mc, &email)?;
    println!("      完成");

    println!("[5/5] 初始化注册表 HKCU\\Software\\Typora ...");
    write_registry().map_err(|e| format!("写注册表失败: {e}"))?;
    println!("      完成\n");

    println!("==== 补丁完成 ====\n");
    println!("接下来:");
    println!("  1. 启动 Typora");
    println!("  2. 在离线激活界面随便填一个 license（如果打开就是已激活状态可忽略）");
    println!("  3. 依次点开[文件、偏好设置、通用]，在右侧关闭 \"自动检查更新\" 与 \"中国大陆服务器\"\n");

    Ok(())
}

fn find_or_ask_install_path() -> Result<PathBuf, String> {
    let default_path = PathBuf::from("C:\\Program Files\\Typora");
    let auto = if default_path.join("Typora.exe").exists() {
        Some(default_path.clone())
    } else {
        None
    };

    if let Some(p) = &auto {
        println!("自动检测到: {}", p.display());
        println!("请输入 Typora 安装目录 (直接回车使用上述检测路径):");
    } else {
        println!("请输入 Typora 安装目录:");
    }

    let line = read_line()?;
    let cleaned = line.trim().trim_matches('"').to_string();
    let p = if cleaned.is_empty() {
        match auto {
            Some(p) => p,
            None => return Err("未提供安装路径".into()),
        }
    } else {
        PathBuf::from(cleaned)
    };

    if !p.join("Typora.exe").exists() {
        return Err(format!("路径下未找到 Typora.exe: {}", p.display()));
    }
    Ok(p)
}

fn read_line() -> Result<String, String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("读取输入失败: {e}"))?;
    Ok(line)
}

fn with_extra_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn inject_hook(launch_dist_js: &Path, mc: &MachineCode, email: &str) -> Result<(), String> {
    let bytes = fs::read(launch_dist_js)
        .map_err(|e| format!("读取 launch.dist.js 失败 ({}): {e}", launch_dist_js.display()))?;
    let content = String::from_utf8(bytes).map_err(|e| format!("launch.dist.js 非 UTF-8: {e}"))?;

    let cfg = hook::HookConfig {
        reg_code: FIXED_REG_CODE,
        date: FIXED_DATE,
        idate: FIXED_IDATE,
        device_id: &mc.l,
        fingerprint: &mc.i,
        email,
        version: &mc.v,
    };
    let injection = hook::build_hook(&cfg);

    let new_content = if let (Some(start), Some(end)) =
        (content.find(hook::HOOK_START), content.find(hook::HOOK_END))
    {
        if end > start {
            let after_end = end + hook::HOOK_END.len();
            let mut s = String::with_capacity(content.len() + injection.len());
            s.push_str(&content[..start]);
            s.push_str(&injection);
            s.push_str(&content[after_end..]);
            s
        } else {
            return Err("已存在 hook 标记但范围非法".into());
        }
    } else {
        let mut idx: Option<usize> = None;
        let mut search_from = 0usize;
        while let Some(rel) = content[search_from..].find("require(") {
            let abs = search_from + rel;
            if let Some(close) = content[abs..].find(");") {
                idx = Some(abs + close + 2);
                break;
            } else {
                search_from = abs + "require(".len();
            }
        }
        let insert_pos = idx.ok_or_else(|| "找不到 require(...) 注入点".to_string())?;
        let mut s = String::with_capacity(content.len() + injection.len());
        s.push_str(&content[..insert_pos]);
        s.push_str(&injection);
        s.push_str(&content[insert_pos..]);
        s
    };

    fs::write(launch_dist_js, new_content.as_bytes())
        .map_err(|e| format!("写 launch.dist.js 失败: {e}"))?;
    Ok(())
}

fn write_registry() -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey_with_flags("Software\\Typora", KEY_SET_VALUE)?;
    let encoded = B64.encode(FIXED_REG_CODE.as_bytes());
    let s_license = format!("{encoded}#0#{FIXED_DATE}");
    key.set_value("SLicense", &s_license)?;
    key.set_value("IDate", &FIXED_IDATE.to_string())?;
    Ok(())
}
