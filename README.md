# Axum PATCH APIでnullと未指定を区別する

AxumとSerdeによるJSON更新APIで、`nickname: null`を「削除」として扱う契約が、`Option<String>`だけでは守れない不具合を再現します。HTTP境界テスト、実行ログ、GDB、コードリーディング、最小修正、回帰テストを含むデバッグラボです。

## この題材で守る契約

| JSON入力 | 意味 | 期待する保存状態 |
| --- | --- | --- |
| `{}` | 更新しない | `Some("taro")`を維持 |
| `{"nickname":null}` | ニックネームを消去する | `None` |
| `{"nickname":"hanako"}` | ニックネームを置換する | `Some("hanako")` |

`Option<String>`では未指定と`null`がどちらも`None`になり、消去と更新なしを区別できません。修正後は`Option<Option<String>>`とカスタムデシリアライズを用い、未指定を`None`、`null`を`Some(None)`として扱います。[1]

## 実行

```bash
cargo fmt --check
cargo test -- --nocapture
```

バグ状態は`71214b8`です。作業中の変更を退避したうえで、次を実行すると`nickname:null`の契約テストが、200だが保存済み値が残るという理由で失敗します。

```bash
git switch --detach 71214b8
cargo test null_nickname_must_clear_the_persisted_value -- --nocapture
git switch main
```

## 構成

```text
src/lib.rs                  Axum API、DTO、HTTP境界テスト
README.md                   再現手順
docs/topic-brief.md         契約と設計
docs/debugging-record.md    観測・原因・修正・回帰範囲
```

## References

[1] [Serdeのnullと未指定を区別する公式サポート例](https://github.com/serde-rs/serde/issues/984)
