# デバッグ記録: PATCHのnullと未指定を区別する

## 契約

`PATCH /profile`へ`{"nickname":null}`を送った場合、HTTP 200と`nickname: null`を返し、保存済みプロフィールのニックネームも消去する。キーが未指定なら現在値を維持する。

## バグ状態の観測

`71214b8`で次を実行しました。

```bash
cargo test null_nickname_must_clear_the_persisted_value -- --nocapture
```

```text
[api] deserialized nickname=None
[api] persisted nickname=Some("taro")
nickname:nullは200とnickname=nullを返し、保存済みのnicknameも消去する必要があります:
status=200 OK, response=Some("taro"), persisted=Some("taro")
```

入力は有効なJSONで、応答は200です。しかしレスポンスと保存済み状態がどちらも古い値を維持しています。

GDBで`src/lib.rs:60`の`if let Some(nickname) = request.nickname`に停止すると、ローカル変数は`request = UpdateProfileRequest { nickname: None }`でした。`null`が更新有無を表す外側の値まで失っており、分岐が実行されないことを確認しました。

## 原因

更新DTOの`nickname: Option<String>`は、キー未指定とJSON `null`をどちらも`None`として扱います。その後の`if let Some`は文字列だけを更新対象にするため、`null`は未指定と同じく何もしません。Serdeの公式サポート例も、未指定を`None`、存在する`null`を`Some(None)`として区別するために、`Option<Option<T>>`とカスタムデシリアライズを用いています。[1]

## 最小修正

DTOを`Option<Option<String>>`に変更し、`#[serde(default, deserialize_with = "deserialize_present_option")]`で未指定は`None`、存在する値は`Some(...)`にします。ハンドラーは`Some(None)`で消去、`Some(Some(value))`で置換、`None`で維持します。

修正コミット`badad13`後には、次のログを確認しました。

```text
[api] deserialized nickname=Some(None)
[api] persisted nickname=None
```

## 回帰範囲

| テスト | 確認すること |
| --- | --- |
| `null_nickname_must_clear_the_persisted_value` | nullで消去する |
| `omitted_nickname_must_preserve_the_persisted_value` | 未指定で維持する |
| `string_nickname_must_replace_the_persisted_value` | 文字列で置換する |

`cargo fmt --check`と`cargo test -- --nocapture`は修正済み状態で成功しました。

## 制約

このラボは単一フィールドの更新意図を扱います。複数フィールドへ同じ規則を適用する場合は、各フィールドのAPI契約を明確にし、共通DTOまたは専用のトライステート型を検討してください。

## References

[1] [Serdeのnullと未指定を区別する公式サポート例](https://github.com/serde-rs/serde/issues/984)
