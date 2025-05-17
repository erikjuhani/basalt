#[test]
fn test_config() {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::{
        keybinding::{Command, Key, KeyBinding},
        Config, TomlConfig,
    };

    let dummy_toml = r#"
        [[key_bindings]]
        key = "page_down"
        command = "page_down"

        [[key_bindings]]
        key = "page_up"
        command = "page_up"
    "#;
    let dummy_toml_config: TomlConfig = toml::from_str::<TomlConfig>(dummy_toml).unwrap();
    let expected_toml_config = TomlConfig {
        key_bindings: Some(Vec::from([
            KeyBinding {
                key: Key {
                    code: KeyCode::PageDown,
                    modifiers: KeyModifiers::NONE,
                },
                command: Command::PageDown,
            },
            KeyBinding {
                key: Key {
                    code: KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                },
                command: Command::PageUp,
            },
        ])),
    };

    assert_eq!(dummy_toml_config, expected_toml_config);

    let mut expected_config = Config::default();
    expected_config.merge_key_bindings(Some(Vec::from([
        KeyBinding {
            key: Key {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            },
            command: Command::PageUp,
        },
        KeyBinding {
            key: Key {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            },
            command: Command::PageDown,
        },
    ])));

    assert_eq!(Config::from(dummy_toml_config), expected_config);
}
