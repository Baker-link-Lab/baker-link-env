---
name: dioxus-component
description: "Dioxus 0.7 RSXコンポーネント作成。Use when: 新しいUIコンポーネント追加、カード追加、モーダル追加、フォーム追加、Dioxus RSX、use_signal、use_coroutine、AppAction"
---

# Dioxus コンポーネント作成

## When to Use
- 新しいUIカード・セクション・モーダル・フォームを追加するとき
- 既存UIの拡張やリファクタリング
- Dioxus RSX のパターンに関する質問

## Architecture Overview

このアプリは単一 `App()` コンポーネント内に状態とUIを集約するアーキテクチャをとる。

```
App() コンポーネント
├── use_hook         → 初回マウント時のみ（トレイ初期化など）
├── use_signal       → リアクティブ状態
├── use_coroutine    → AppAction ディスパッチャー（副作用の一元管理）
├── use_effect       → 状態変更に反応するリアクティブ副作用
├── use_future       → ポーリング・バックグラウンドタスク
└── rsx! { ... }     → UI記述
```

## Procedure

### 1. 状態の追加

`App()` 内の既存 signal 群の直後に追加する。命名は snake_case:

```rust
let mut my_value = use_signal(|| "default".to_string());
let mut show_my_modal = use_signal(|| false);
```

**型パターン:**
- 文字列入力: `use_signal(|| "default".to_string())`
- Boolean フラグ: `use_signal(|| false)`
- Optional エラー: `use_signal(|| Option::<String>::None)`
- リスト: `use_signal(Vec::<T>::new)`

### 2. 副作用の追加（必要な場合）

副作用は必ず `AppAction` enum + `use_coroutine` パターンに従う。直接 `spawn()` しない。

```rust
// 1. AppAction に variant を追加
enum AppAction {
    // ...既存...
    MyNewAction(String),
}

// 2. coroutine の match に処理を追加
AppAction::MyNewAction(param) => {
    // 非同期処理やグローバル状態アクセスはここで
    crate::log_info(format!("Action executed: {}", param));
}
```

**非同期外部コマンドのパターン** (UIスレッドをブロックしない):
```rust
AppAction::MyAction => {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(cmd::some_function());
    });
    match rx.await {
        Ok(Ok(result)) => { /* success */ }
        Ok(Err(e)) => { last_error.set(Some(e)); }
        Err(_) => { crate::log_error("channel closed"); }
    }
}
```

### 3. RSX UI の記述

#### カードコンポーネント
```rust
section { class: "card",
    h2 { class: "section-title", "Card Title" }
    p { class: "section-subtitle", "Description text." }
    div { class: "card-body",
        // コンテンツ
    }
}
```

#### 入力フォーム行
```rust
div { class: "input-row",
    label { class: "input-label", "Label" }
    input {
        class: "input",
        value: "{my_value}",
        oninput: move |ev| my_value.set(ev.value()),
    }
    button {
        class: "btn-primary",
        onclick: move |_| actions.send(AppAction::MyNewAction(my_value.read().clone())),
        "Execute"
    }
}
```

#### モーダルダイアログ
```rust
if *show_my_modal.read() {
    div { class: "modal-overlay",
        div { class: "modal",
            h3 { class: "modal-title", "Title" }
            p { class: "modal-text", "Message content." }
            div { class: "modal-actions",
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        // 処理
                        show_my_modal.set(false);
                    },
                    "OK"
                }
                button {
                    class: "btn-chip",
                    onclick: move |_| show_my_modal.set(false),
                    "Cancel"
                }
            }
        }
    }
}
```

#### エラートースト
```rust
if let Some(err) = last_error.read().clone() {
    div { class: "error-toast",
        div { class: "error-toast-inner",
            span { "[ERROR] {err}" }
            button {
                class: "error-dismiss",
                onclick: move |_| last_error.set(None),
                "\u{00d7}"
            }
        }
    }
}
```

### 4. CSS スタイリング

`assets/main.css` に追加。CSS変数プレフィックスは `--bkl-`:

```css
.my-component {
    background: var(--bkl-card-bg);
    border-left: 3px solid var(--bkl-orange);
    border-radius: var(--bkl-radius);
    padding: var(--bkl-card-padding);
}
```

**利用可能な主要 CSS クラス:**
- ボタン: `.btn-primary`(orange), `.btn-danger`(red), `.btn-chip`(subtle)
- カード: `.card`, `.card-full`(全幅)
- 入力: `.input`, `.input-narrow`
- ステータス: `.status-dot`, `.status-dot-pulse`, `.status-dot-green`, `.status-dot-red`
- レイアウト: `.cards-grid`(2列), `.main-content`

### 5. ポーリング（定期実行）

```rust
use_future(move || async move {
    loop {
        // 処理
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
});
```

## Quality Checklist

- [ ] 状態は `use_signal` で管理している
- [ ] 副作用は `AppAction` + `use_coroutine` 経由
- [ ] 外部コマンド実行は `std::thread::spawn` + `oneshot::channel` でUIをブロックしない
- [ ] ログ出力は `crate::log_info()` / `crate::log_error()` を使用
- [ ] CSSクラスは `assets/main.css` に追加し、`--bkl-` プレフィックスの変数を使う
- [ ] エラーは `last_error` signal に設定してトーストで表示
