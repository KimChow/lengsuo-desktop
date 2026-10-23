//! `state::hotkeys` 的单测（原 `hotkeys.rs` 第 468–687 行，逻辑一字未改）。

use super::*;

use crate::theme::palette::COMMANDS;

fn hotkey(spec: &str) -> Hotkey {
    Hotkey::parse(spec).expect("测试快捷键应可解析")
}

#[test]
fn parses_documented_shortcuts() {
    assert_eq!(
        hotkey("Ctrl+K"),
        Hotkey::new(Modifiers::CTRL, Key::Character('k'))
    );
    assert_eq!(
        hotkey("Ctrl+Shift+P"),
        Hotkey::new(Modifiers::CTRL_SHIFT, Key::Character('p'))
    );
    assert_eq!(hotkey("Escape"), Hotkey::plain(Key::Escape));
}

#[test]
fn parsing_ignores_case_order_and_spaces() {
    assert_eq!(hotkey("shift + ctrl + p"), hotkey("Ctrl+Shift+P"));
    assert_eq!(hotkey("CTRL+k"), hotkey("ctrl+K"));
    assert_eq!(hotkey("esc"), hotkey("Escape"));
}

#[test]
fn invalid_specs_are_rejected() {
    assert_eq!(Hotkey::parse(""), Err(HotkeyParseError::Empty));
    assert!(matches!(
        Hotkey::parse("Ctrl"),
        Err(HotkeyParseError::MissingKey)
    ));
    assert!(matches!(
        Hotkey::parse("Ctrl+K+J"),
        Err(HotkeyParseError::MultipleKeys(_))
    ));
    assert!(matches!(
        Hotkey::parse("Hyper+K"),
        Err(HotkeyParseError::UnknownKey(_))
    ));
    assert!(matches!(
        Hotkey::parse("Ctrl+Ctrl+K"),
        Err(HotkeyParseError::DuplicateModifier(_))
    ));
    assert!(matches!(
        Hotkey::parse("Ctrl+"),
        Err(HotkeyParseError::Malformed(_))
    ));
}

#[test]
fn display_is_canonical() {
    assert_eq!(hotkey("shift+ctrl+p").to_string(), "Ctrl+Shift+P");
    assert_eq!(hotkey("esc").to_string(), "Escape");
    assert_eq!(hotkey("F5").to_string(), "F5");
}

#[test]
fn defaults_match_documented_shortcuts() {
    let registry = HotkeyRegistry::with_defaults().expect("内置快捷键表必须无冲突");
    assert_eq!(
        registry.command_for(hotkey("Ctrl+K")),
        Some(command_ids::PALETTE_OPEN)
    );
    assert_eq!(
        registry.command_for(hotkey("Ctrl+Shift+P")),
        Some(command_ids::PALETTE_OPEN)
    );
    assert_eq!(
        registry.command_for(hotkey("Ctrl+S")),
        Some(command_ids::FILE_SAVE)
    );
    assert_eq!(
        registry.command_for(hotkey("Escape")),
        Some(command_ids::OVERLAY_CLOSE)
    );
}

#[test]
fn matching_requires_exact_modifiers() {
    let registry = HotkeyRegistry::with_defaults().unwrap();
    assert_eq!(registry.command_for(hotkey("Ctrl+Shift+K")), None);
    assert_eq!(registry.command_for(hotkey("K")), None);
    assert_eq!(registry.command_for(hotkey("Alt+K")), None);
}

#[test]
fn two_keys_for_one_command_are_not_a_conflict() {
    let registry = HotkeyRegistry::with_defaults().unwrap();
    let count = registry
        .bindings()
        .iter()
        .filter(|binding| binding.command_id == command_ids::PALETTE_OPEN)
        .count();
    assert_eq!(count, 2, "一个命令可以有两个键，不算冲突");
}

#[test]
fn duplicate_hotkey_errors_instead_of_taking_first() {
    let result = HotkeyRegistry::from_bindings([
        Binding {
            hotkey: hotkey("Ctrl+K"),
            command_id: "session.new",
        },
        Binding {
            hotkey: hotkey("Ctrl+K"),
            command_id: "session.list",
        },
    ]);
    match result {
        Err(HotkeyError::Conflict {
            hotkey,
            first,
            second,
        }) => {
            assert_eq!(hotkey.to_string(), "Ctrl+K");
            assert_eq!((first, second), ("session.new", "session.list"));
        }
        other => panic!("必须报冲突，不得静默取第一个：{other:?}"),
    }
}

#[test]
fn bind_rejects_conflict_and_leaves_registry_untouched() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    let error = registry.bind(hotkey("Ctrl+S"), "session.new").unwrap_err();
    assert!(matches!(error, HotkeyError::Conflict { .. }));
    assert_eq!(
        registry.command_for(hotkey("Ctrl+S")),
        Some(command_ids::FILE_SAVE),
        "冲突失败后原绑定必须保持"
    );
}

#[test]
fn dispatch_uses_id_not_title() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    registry.bind(hotkey("Ctrl+N"), "session.new").unwrap();
    let id = registry.command_for(hotkey("Ctrl+N")).unwrap();
    assert_eq!(id, "session.new");
    let command = COMMANDS.iter().find(|command| command.id == id).unwrap();
    assert_ne!(id, command.title, "派发依据是 id，不是标题");
}

#[test]
fn user_override_conflict_is_detected() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    let overrides = [HotkeyOverride::parse("session.new", "Ctrl+K").unwrap()];
    let error = registry.apply_overrides(&overrides).unwrap_err();
    assert!(matches!(error, HotkeyError::Conflict { .. }));
    assert_eq!(
        registry.command_for(hotkey("Ctrl+K")),
        Some(command_ids::PALETTE_OPEN),
        "覆盖失败时注册表不得改变"
    );
    assert_eq!(registry.command_for(hotkey("Ctrl+N")), None);
}

#[test]
fn successful_override_is_applied_and_revalidated() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    registry
        .apply_overrides(&[HotkeyOverride::parse("session.new", "Ctrl+N").unwrap()])
        .unwrap();
    assert_eq!(registry.command_for(hotkey("Ctrl+N")), Some("session.new"));
    // 再改成已占用的键：仍然必须报错。
    let error = registry
        .apply_overrides(&[HotkeyOverride::parse("session.new", "Ctrl+S").unwrap()])
        .unwrap_err();
    assert!(matches!(error, HotkeyError::Conflict { .. }));
}

#[test]
fn override_releases_the_old_key() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    registry
        .rebind("file.save", hotkey("Ctrl+Shift+S"))
        .unwrap();
    assert_eq!(
        registry.command_for(hotkey("Ctrl+Shift+S")),
        Some(command_ids::FILE_SAVE)
    );
    assert_eq!(
        registry.command_for(hotkey("Ctrl+S")),
        None,
        "改键后旧键必须释放"
    );
}

#[test]
fn unknown_command_is_rejected() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    assert!(matches!(
        registry.apply_overrides(&[HotkeyOverride::parse("nope.cmd", "Ctrl+N").unwrap()]),
        Err(HotkeyError::UnknownCommand(_))
    ));
    assert!(matches!(
        registry.bind(hotkey("Ctrl+N"), "nope.cmd"),
        Err(HotkeyError::UnknownCommand(_))
    ));
}

#[test]
fn override_batch_is_atomic() {
    let mut registry = HotkeyRegistry::with_defaults().unwrap();
    let before = registry.bindings().to_vec();
    let overrides = [
        HotkeyOverride::parse("session.new", "Ctrl+N").unwrap(),
        HotkeyOverride::parse("session.list", "Ctrl+K").unwrap(),
    ];
    let error = registry.apply_overrides(&overrides).unwrap_err();
    assert_eq!(registry.bindings(), before, "整批覆盖失败时不得部分生效");
    let text = error.to_string();
    assert!(text.contains("session.list") && text.contains("palette.open"));
    assert!(text.contains("Ctrl+K"));
}
