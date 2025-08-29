# CreateM2MToken201Response

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** |  | 
**id** | **String** |  | 
**subject** | **String** |  | 
**claims** | Option<[**serde_json::Value**](.md)> |  | [optional]
**scopes** | Option<**Vec<String>**> |  | [optional][default to []]
**token** | **String** |  | 
**revoked** | **bool** |  | 
**revocation_reason** | Option<**String**> |  | 
**expired** | **bool** |  | 
**expiration** | Option<**f64**> |  | 
**last_used_at** | Option<**f64**> |  | 
**created_at** | **f64** |  | 
**updated_at** | **f64** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


