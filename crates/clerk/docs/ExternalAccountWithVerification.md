# ExternalAccountWithVerification

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** | String representing the object's type. Objects of the same type share the same value. | 
**id** | **String** |  | 
**provider** | **String** |  | 
**identification_id** | **String** |  | 
**provider_user_id** | **String** | The unique ID of the user in the external provider's system | 
**approved_scopes** | **String** |  | 
**email_address** | **String** |  | 
**first_name** | **String** |  | 
**last_name** | **String** |  | 
**avatar_url** | Option<**String**> | Please use `image_url` instead | [optional]
**image_url** | Option<**String**> |  | [optional]
**username** | Option<**String**> |  | [optional]
**phone_number** | Option<**String**> |  | [optional]
**public_metadata** | [**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md) |  | 
**label** | Option<**String**> |  | [optional]
**created_at** | **i64** | Unix timestamp of creation  | 
**updated_at** | **i64** | Unix timestamp of creation  | 
**verification** | Option<[**models::ExternalAccountWithVerificationVerification**](ExternalAccountWithVerification_verification.md)> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


