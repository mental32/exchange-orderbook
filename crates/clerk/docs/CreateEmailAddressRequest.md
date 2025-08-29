# CreateEmailAddressRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**user_id** | **String** | The ID representing the user | 
**email_address** | **String** | The new email address. Must adhere to the RFC 5322 specification for email address format. | 
**verified** | Option<**bool**> | When created, the email address will be marked as verified. | [optional]
**primary** | Option<**bool**> | Create this email address as the primary email address for the user. Default: false, unless it is the first email address. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


