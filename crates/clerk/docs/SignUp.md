# SignUp

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** |  | 
**id** | **String** |  | 
**status** | **String** |  | 
**required_fields** | **Vec<String>** |  | 
**optional_fields** | **Vec<String>** |  | 
**missing_fields** | **Vec<String>** |  | 
**unverified_fields** | **Vec<String>** |  | 
**verifications** | [**models::SignUpVerifications**](SignUpVerifications.md) |  | 
**username** | Option<**String**> |  | 
**email_address** | Option<**String**> |  | 
**phone_number** | Option<**String**> |  | 
**web3_wallet** | Option<**String**> |  | 
**password_enabled** | **bool** |  | 
**first_name** | Option<**String**> |  | 
**last_name** | Option<**String**> |  | 
**unsafe_metadata** | Option<[**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md)> |  | [optional]
**public_metadata** | Option<[**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md)> |  | [optional]
**custom_action** | **bool** |  | 
**external_id** | Option<**String**> |  | 
**created_session_id** | Option<**String**> |  | 
**created_user_id** | Option<**String**> |  | 
**abandon_at** | **i64** | Unix timestamp at which the user abandoned the sign up attempt.  | 
**legal_accepted_at** | Option<**i64**> | Unix timestamp at which the user accepted the legal requirements.  | 
**external_account** | Option<[**serde_json::Value**](.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


