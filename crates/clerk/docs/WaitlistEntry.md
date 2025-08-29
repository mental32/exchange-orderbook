# WaitlistEntry

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** |  | 
**id** | **String** |  | 
**email_address** | **String** |  | 
**status** | **String** |  | 
**is_locked** | Option<**bool**> | Indicates if the waitlist entry is locked. Locked entries are being processed in a batch action and are unavailable for other actions.  | [optional]
**created_at** | **i64** | Unix timestamp of creation.  | 
**updated_at** | **i64** | Unix timestamp of last update.  | 
**invitation** | Option<[**models::Invitation**](Invitation.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


