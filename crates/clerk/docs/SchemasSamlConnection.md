# SchemasSamlConnection

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** |  | 
**id** | **String** |  | 
**name** | **String** |  | 
**domain** | Option<**String**> |  | [optional]
**domains** | Option<**Vec<String>**> |  | [optional]
**idp_entity_id** | Option<**String**> |  | 
**idp_sso_url** | Option<**String**> |  | 
**idp_certificate** | Option<**String**> |  | 
**idp_metadata_url** | Option<**String**> |  | [optional]
**idp_metadata** | Option<**String**> |  | [optional]
**acs_url** | **String** |  | 
**sp_entity_id** | **String** |  | 
**sp_metadata_url** | **String** |  | 
**organization_id** | Option<**String**> |  | [optional]
**attribute_mapping** | Option<[**models::SamlConnectionAttributeMapping**](SAMLConnectionAttributeMapping.md)> |  | [optional]
**active** | **bool** |  | 
**provider** | **String** |  | 
**user_count** | **i32** |  | 
**sync_user_attributes** | **bool** |  | 
**allow_subdomains** | Option<**bool**> |  | [optional]
**allow_idp_initiated** | Option<**bool**> |  | [optional]
**disable_additional_identifications** | Option<**bool**> |  | [optional]
**created_at** | **i64** | Unix timestamp of creation.  | 
**updated_at** | **i64** | Unix timestamp of last update.  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


