# デバッグ記録: PATCHのnullと未指定を区別する

## 実行環境と再現境界

| 項目 | 内容 |
| --- | --- |
| 言語と主要ライブラリ | Rust、Axum、Serde、Tokio |
| 再現境界 | Axum `Router`へ`oneshot`で送る`PATCH /profile` |
| 入力 | `Content-Type: application/json`と`{"nickname":null}` |
| 最終観測 | HTTP応答JSONと`AppState`に保存されたプロフィール |
| 決定性 | インメモリ状態のみを使い、外部ネットワーク・DB・時刻を使わない |

## 最初に観測した事実

`71214b8`で`cargo test null_nickname_must_clear_the_persisted_value -- --nocapture`を実行しました。

```text
[api] deserialized nickname=None
[api] persisted nickname=Some("taro")
status=200 OK, response=Some("taro"), persisted=Some("taro")
```

有効なJSONを送ってHTTP 200を受け取りましたが、レスポンスと保存済み状態の両方に古いニックネームが残りました。

## 競合仮説と検証

| 仮説 | 検証 | 結果 |
| --- | --- | --- |
| JSONまたはContent-Typeが不正 | 有効JSONと`application/json`を固定する | 200のため除外 |
| ハンドラーが実行されない | DTOを出力するログを確認する | ログがあるため除外 |
| nullが未指定と同じDTO値になる | GDBで更新分岐の直前に停止する | 支持 |
| 保存だけが失敗する | DTO・応答・保存済み状態を別々に確認する | DTOが更新意図を失うため除外 |

GDBで`src/lib.rs:60`に停止すると、`request = UpdateProfileRequest { nickname: None }`が確認できました。

## 確定した原因

更新DTOの`nickname: Option<String>`では、キー未指定とJSON `null`がどちらも`None`になります。続く`if let Some`は文字列だけを置換対象にするため、`null`は未指定と同じく更新を省略します。Serdeの公式サポート例も、未指定を`None`、存在する`null`を`Some(None)`として扱う方法を示しています。[1]

## 最小修正

DTOを`Option<Option<String>>`へ変更し、`#[serde(default, deserialize_with = "deserialize_present_option")]`で未指定と存在する値を区別しました。ハンドラーは`Some(None)`で消去、`Some(Some(value))`で置換、`None`で維持します。

修正コミット`badad13`では次のログになりました。

```text
[api] deserialized nickname=Some(None)
[api] persisted nickname=None
```

## 回帰保証

| テスト | 守る契約 |
| --- | --- |
| `null_nickname_must_clear_the_persisted_value` | nullで消去する |
| `omitted_nickname_must_preserve_the_persisted_value` | 未指定で維持する |
| `string_nickname_must_replace_the_persisted_value` | 文字列で置換する |

修正済み状態で`cargo fmt --check`と`cargo test -- --nocapture`が成功しました。

## 再現手順

```bash
# 修正済み状態を検証する
cargo fmt --check
cargo test -- --nocapture

# バグ状態を確認する。作業中の変更は先に退避する
git switch --detach 71214b8
cargo test null_nickname_must_clear_the_persisted_value -- --nocapture
git switch main
```

## スコープと注意点

このラボは単一の任意文字列フィールドを扱います。複数フィールドのPATCH APIでは、未指定・null・空文字列それぞれの契約をフィールドごとに明記してください。入力DTOと永続化モデルを分けると、transport層の三状態を失わずに扱えます。

## References

[1] [Serdeのnullと未指定を区別する公式サポート例](https://github.com/serde-rs/serde/issues/984)
