# OAuthAccessTokenInner

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** |  | 
**external_account_id** | **String** | External account ID | 
**provider_user_id** | **String** | The unique ID of the user in the external provider's system | 
**token** | **String** | The access token | 
**expires_at** | Option<**i64**> | Unix timestamp of the access token expiration. | 
**provider** | **String** | The ID of the provider | 
**public_metadata** | [**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md) |  | 
**label** | Option<**String**> |  | 
**scopes** | Option<**Vec<String>**> | The list of scopes that the token is valid for. Only present for OAuth 2.0 tokens. | [optional]
**token_secret** | Option<**String**> | The token secret. Only present for OAuth 1.0 tokens. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


