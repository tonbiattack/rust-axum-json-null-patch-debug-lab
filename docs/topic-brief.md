# 題材企画: PATCHのnullと未指定が区別できない

| 項目 | 内容 |
| --- | --- |
| 対象 | Rust、Axum、SerdeによるJSON PATCH API |
| 契約 | `nickname:null`は保存済みニックネームを消去し、キー未指定は現在値を維持する |
| API境界 | `PATCH /profile`をAxum `Router`へ`oneshot`で送る |
| 最終観測 | HTTP応答のJSONと`AppState`の保存済みプロフィールを別々に確認する |
| バグ状態 | `71214b8`。`Option<String>`によりnullと未指定がどちらも`None`になる |
| 修正状態 | `badad13`。`Option<Option<String>>`とカスタムデシリアライズで三状態を保持する |

## 仮説

| 仮説 | 確認方法 |
| --- | --- |
| JSONのnullがDTOで未指定と同じ値になる | APIログとGDBで`request.nickname`を確認する |
| ハンドラーの更新ロジックがnullを消去しない | 200応答と保存済み値を個別に確認する |
| リクエストの形式が不正 | `application/json`と有効JSONを固定し、修正後の同じ入力が成功することを確認する |
