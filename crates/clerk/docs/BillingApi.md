# \BillingApi

All URIs are relative to *https://api.clerk.com/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_organization_billing_subscription**](BillingApi.md#get_organization_billing_subscription) | **GET** /organizations/{organization_id}/billing/subscription | Retrieve an organization's billing subscription
[**get_user_billing_subscription**](BillingApi.md#get_user_billing_subscription) | **GET** /users/{user_id}/billing/subscription | Retrieve a user's billing subscription



## get_organization_billing_subscription

> models::CommerceSubscription get_organization_billing_subscription(organization_id)
Retrieve an organization's billing subscription

Retrieves the billing subscription for the specified organization. This includes subscription details, active plans, billing information, and payment status. The subscription contains subscription items which represent the individual plans the organization is subscribed to.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**organization_id** | **String** | The ID of the organization whose subscription to retrieve | [required] |

### Return type

[**models::CommerceSubscription**](CommerceSubscription.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_user_billing_subscription

> models::CommerceSubscription get_user_billing_subscription(user_id)
Retrieve a user's billing subscription

Retrieves the billing subscription for the specified user. This includes subscription details, active plans, billing information, and payment status. The subscription contains subscription items which represent the individual plans the user is subscribed to.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose subscription to retrieve | [required] |

### Return type

[**models::CommerceSubscription**](CommerceSubscription.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

