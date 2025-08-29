# \AwsCredentialsApi

All URIs are relative to *https://api.clerk.com/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**delete_aws_credential**](AwsCredentialsApi.md#delete_aws_credential) | **DELETE** /aws_credentials/{id} | Delete an AWS Credential
[**update_aws_credential**](AwsCredentialsApi.md#update_aws_credential) | **PATCH** /aws_credentials/{id} | Update an AWS Credential



## delete_aws_credential

> models::DeletedObject delete_aws_credential(id)
Delete an AWS Credential

Delete the AWS Credential with the given ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | The ID of the AWS Credential to delete | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_aws_credential

> models::AwsCredential update_aws_credential(id, update_aws_credential_request)
Update an AWS Credential

Updates an AWS credential.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | The ID of the AWS Credential to update | [required] |
**update_aws_credential_request** | Option<[**UpdateAwsCredentialRequest**](UpdateAwsCredentialRequest.md)> |  |  |

### Return type

[**models::AwsCredential**](AWSCredential.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

