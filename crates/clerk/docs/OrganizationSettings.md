# OrganizationSettings

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** | String representing the object's type. Objects of the same type share the same value. | 
**enabled** | **bool** |  | 
**max_allowed_memberships** | **i32** |  | 
**max_allowed_roles** | **i32** |  | 
**max_allowed_permissions** | Option<**i32**> | max_allowed_permissions is now a no-op, as permissions are now unlimited | [optional]
**creator_role** | **String** | The role key that a user will be assigned after creating an organization. | 
**admin_delete_enabled** | **bool** | The default for whether an admin can delete an organization with the Frontend API. | 
**domains_enabled** | **bool** |  | 
**domains_enrollment_modes** | **Vec<String>** |  | 
**domains_default_role** | **String** | The role key that it will be used in order to create an organization invitation or suggestion. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


