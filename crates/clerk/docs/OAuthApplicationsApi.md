# \OAuthApplicationsApi

All URIs are relative to *https://api.clerk.com/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_o_auth_application**](OAuthApplicationsApi.md#create_o_auth_application) | **POST** /oauth_applications | Create an OAuth application
[**delete_o_auth_application**](OAuthApplicationsApi.md#delete_o_auth_application) | **DELETE** /oauth_applications/{oauth_application_id} | Delete an OAuth application
[**get_o_auth_application**](OAuthApplicationsApi.md#get_o_auth_application) | **GET** /oauth_applications/{oauth_application_id} | Retrieve an OAuth application by ID
[**list_o_auth_applications**](OAuthApplicationsApi.md#list_o_auth_applications) | **GET** /oauth_applications | Get a list of OAuth applications for an instance
[**rotate_o_auth_application_secret**](OAuthApplicationsApi.md#rotate_o_auth_application_secret) | **POST** /oauth_applications/{oauth_application_id}/rotate_secret | Rotate the client secret of the given OAuth application
[**update_o_auth_application**](OAuthApplicationsApi.md#update_o_auth_application) | **PATCH** /oauth_applications/{oauth_application_id} | Update an OAuth application



## create_o_auth_application

> models::OAuthApplicationWithSecret create_o_auth_application(create_o_auth_application_request)
Create an OAuth application

Creates a new OAuth application with the given name and callback URL for an instance. The callback URL must be a valid url. All URL schemes are allowed such as `http://`, `https://`, `myapp://`, etc...

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_o_auth_application_request** | Option<[**CreateOAuthApplicationRequest**](CreateOAuthApplicationRequest.md)> |  |  |

### Return type

[**models::OAuthApplicationWithSecret**](OAuthApplicationWithSecret.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_o_auth_application

> models::DeletedObject delete_o_auth_application(oauth_application_id)
Delete an OAuth application

Deletes the given OAuth application. This is not reversible.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**oauth_application_id** | **String** | The ID of the OAuth application to delete | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_o_auth_application

> models::OAuthApplication get_o_auth_application(oauth_application_id)
Retrieve an OAuth application by ID

Fetches the OAuth application whose ID matches the provided `id` in the path.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**oauth_application_id** | **String** | The ID of the OAuth application | [required] |

### Return type

[**models::OAuthApplication**](OAuthApplication.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_o_auth_applications

> models::OAuthApplications list_o_auth_applications(limit, offset, order_by, name_query)
Get a list of OAuth applications for an instance

This request returns the list of OAuth applications for an instance. Results can be paginated using the optional `limit` and `offset` query parameters. The OAuth applications are ordered by descending creation date. Most recent OAuth applications will be returned first.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]
**order_by** | Option<**String**> | Allows to return OAuth applications in a particular order. At the moment, you can order the returned OAuth applications by their `created_at` and `name`. In order to specify the direction, you can use the `+/-` symbols prepended in the property to order by. For example, if you want OAuth applications to be returned in descending order according to their `created_at` property, you can use `-created_at`. If you don't use `+` or `-`, then `+` is implied. We only support one `order_by` parameter, and if multiple `order_by` parameters are provided, we will only keep the first one. For example, if you pass `order_by=name&order_by=created_at`, we will consider only the first `order_by` parameter, which is `name`. The `created_at` parameter will be ignored in this case. |  |[default to +created_at]
**name_query** | Option<**String**> | Returns OAuth applications with names that match the given query, via case-insensitive partial match. |  |

### Return type

[**models::OAuthApplications**](OAuthApplications.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## rotate_o_auth_application_secret

> models::OAuthApplicationWithSecret rotate_o_auth_application_secret(oauth_application_id)
Rotate the client secret of the given OAuth application

Rotates the OAuth application's client secret. When the client secret is rotated, make sure to update it in authorized OAuth clients.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**oauth_application_id** | **String** | The ID of the OAuth application for which to rotate the client secret | [required] |

### Return type

[**models::OAuthApplicationWithSecret**](OAuthApplicationWithSecret.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_o_auth_application

> models::OAuthApplication update_o_auth_application(oauth_application_id, update_o_auth_application_request)
Update an OAuth application

Updates an existing OAuth application

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**oauth_application_id** | **String** | The ID of the OAuth application to update | [required] |
**update_o_auth_application_request** | [**UpdateOAuthApplicationRequest**](UpdateOAuthApplicationRequest.md) |  | [required] |

### Return type

[**models::OAuthApplication**](OAuthApplication.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

