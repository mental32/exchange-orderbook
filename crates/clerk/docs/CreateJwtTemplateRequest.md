# CreateJwtTemplateRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** | JWT template name | 
**claims** | [**serde_json::Value**](.md) | JWT template claims in JSON format | 
**lifetime** | Option<**i32**> | JWT token lifetime | [optional]
**allowed_clock_skew** | Option<**i32**> | JWT token allowed clock skew | [optional]
**custom_signing_key** | Option<**bool**> | Whether a custom signing key/algorithm is also provided for this template | [optional]
**signing_algorithm** | Option<**String**> | The custom signing algorithm to use when minting JWTs. Required if `custom_signing_key` is `true`. | [optional]
**signing_key** | Option<**String**> | The custom signing private key to use when minting JWTs. Required if `custom_signing_key` is `true`. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


