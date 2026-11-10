//! 启动器窗口尺寸计算：纯函数，便于单测。
//!
//! 尺寸单位是逻辑像素（Tauri 的 LogicalSize），显示器分辨率需要先除以 scale_factor。

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LauncherSize {
    pub width: f64,
    pub base_height: f64,
    pub tall_height: f64,
}

pub const PRESETS: [&str; 4] = ["auto", "compact", "standard", "large"];

pub fn is_valid_preset(preset: &str) -> bool {
    PRESETS.contains(&preset)
}

/// auto 按显示器逻辑宽度的 35% 计算，并限制在 560~900 之间；
/// 高度随宽度走，详情/编辑态额外加 140，且不超过屏幕可用高度。
pub fn compute(preset: &str, monitor_width: f64, monitor_height: f64) -> LauncherSize {
    let (width, base_height) = match preset {
        "compact" => (600.0, 380.0),
        "large" => (800.0, 480.0),
        "auto" => {
            let w = (monitor_width * 0.35).round().clamp(560.0, 900.0);
            let h = (w * 0.62).round().clamp(380.0, 520.0);
            (w, h)
        }
        // standard 及未知值
        _ => (680.0, 420.0),
    };
    let tall = (base_height + 140.0).min((monitor_height - 80.0).max(base_height + 40.0));
    LauncherSize {
        width,
        base_height,
        tall_height: tall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_and_compact_large_are_fixed() {
        let s = compute("standard", 1920.0, 1080.0);
        assert_eq!((s.width, s.base_height, s.tall_height), (680.0, 420.0, 560.0));
        let c = compute("compact", 1920.0, 1080.0);
        assert_eq!((c.width, c.base_height), (600.0, 380.0));
        let l = compute("large", 1920.0, 1080.0);
        assert_eq!((l.width, l.base_height), (800.0, 480.0));
    }

    #[test]
    fn auto_scales_with_monitor() {
        let s = compute("auto", 1920.0, 1080.0);
        assert_eq!((s.width, s.base_height), (672.0, 417.0));
        // 小屏不低于下限
        let small = compute("auto", 1366.0, 768.0);
        assert_eq!((small.width, small.base_height), (560.0, 380.0));
        // 4K@150% 逻辑宽 2560，不超过上限
        let big = compute("auto", 2560.0, 1440.0);
        assert_eq!((big.width, big.base_height), (896.0, 520.0));
    }

    #[test]
    fn tall_height_fits_small_screen() {
        let s = compute("standard", 1366.0, 600.0);
        assert_eq!(s.tall_height, 520.0, "矮屏上详情态应被压到屏幕内");
    }

    #[test]
    fn unknown_preset_falls_back_to_standard() {
        assert_eq!(compute("nope", 1920.0, 1080.0), compute("standard", 1920.0, 1080.0));
        assert!(!is_valid_preset("nope"));
    }
}
