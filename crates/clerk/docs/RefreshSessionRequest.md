# RefreshSessionRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**expired_token** | **String** | The JWT that is sent via the `__session` cookie from your frontend. Note: this JWT must be associated with the supplied session ID. | 
**refresh_token** | **String** | The JWT that is sent via the `__session` cookie from your frontend. | 
**request_origin** | **String** | The origin of the request. | 
**request_headers** | Option<[**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md)> | The headers of the request. | [optional]
**format** | Option<**String**> | The format of the response. | [optional][default to Token]
**request_originating_ip** | Option<**String**> | The IP address of the request. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


