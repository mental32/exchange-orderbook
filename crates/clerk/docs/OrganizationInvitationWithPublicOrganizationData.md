# OrganizationInvitationWithPublicOrganizationData

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** | String representing the object's type. Objects of the same type share the same value.  | 
**id** | **String** |  | 
**email_address** | **String** |  | 
**role** | **String** |  | 
**role_name** | **String** |  | 
**organization_id** | Option<**String**> |  | [optional]
**status** | Option<**String**> |  | [optional]
**public_metadata** | [**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md) |  | 
**private_metadata** | Option<[**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md)> |  | [optional]
**url** | Option<**String**> |  | 
**expires_at** | Option<**i64**> | Unix timestamp of expiration. | 
**created_at** | **i64** | Unix timestamp of creation. | 
**updated_at** | **i64** | Unix timestamp of last update. | 
**public_organization_data** | Option<[**models::OrganizationInvitationPublicOrganizationData**](OrganizationInvitationPublicOrganizationData.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


