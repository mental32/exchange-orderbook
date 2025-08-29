# \CommerceApi

All URIs are relative to *https://api.clerk.com/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancel_commerce_subscription_item**](CommerceApi.md#cancel_commerce_subscription_item) | **DELETE** /commerce/subscription_items/{subscription_item_id} | Cancel a subscription item
[**extend_commerce_subscription_item_free_trial**](CommerceApi.md#extend_commerce_subscription_item_free_trial) | **POST** /billing/subscription_items/{subscription_item_id}/extend_free_trial | Extend free trial for a subscription item
[**get_commerce_plan_list**](CommerceApi.md#get_commerce_plan_list) | **GET** /commerce/plans | List all commerce plans
[**get_commerce_subscription_item_list**](CommerceApi.md#get_commerce_subscription_item_list) | **GET** /commerce/subscription_items | List all subscription items



## cancel_commerce_subscription_item

> models::CommerceSubscriptionItem cancel_commerce_subscription_item(subscription_item_id, end_now)
Cancel a subscription item

Cancel a specific subscription item. The subscription item can be canceled immediately or at the end of the current billing period.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**subscription_item_id** | **String** | The ID of the subscription item to cancel | [required] |
**end_now** | Option<**bool**> | Whether to cancel the subscription immediately (true) or at the end of the current billing period (false, default) |  |[default to false]

### Return type

[**models::CommerceSubscriptionItem**](CommerceSubscriptionItem.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## extend_commerce_subscription_item_free_trial

> models::CommerceSubscriptionItem extend_commerce_subscription_item_free_trial(subscription_item_id, extend_free_trial_request)
Extend free trial for a subscription item

Extends the free trial period for a specific subscription item to the specified timestamp. The subscription item must be currently in a free trial period, and the plan must support free trials. The timestamp must be in the future and not more than 365 days from the end of the current trial period This operation is idempotent - repeated requests with the same timestamp will not change the trial period.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**subscription_item_id** | **String** | The ID of the subscription item to extend the free trial for | [required] |
**extend_free_trial_request** | [**ExtendFreeTrialRequest**](ExtendFreeTrialRequest.md) | Parameters for extending the free trial | [required] |

### Return type

[**models::CommerceSubscriptionItem**](CommerceSubscriptionItem.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_commerce_plan_list

> models::PaginatedCommercePlanResponse get_commerce_plan_list(paginated, limit, offset, payer_type)
List all commerce plans

Returns a list of all commerce plans for the instance. The plans are returned sorted by creation date, with the newest plans appearing first. This includes both free and paid plans. Pagination is supported.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**paginated** | Option<**bool**> | Whether to paginate the results. If true, the results will be paginated. If false, the results will not be paginated. |  |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]
**payer_type** | Option<**String**> | Filter plans by payer type |  |

### Return type

[**models::PaginatedCommercePlanResponse**](PaginatedCommercePlanResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_commerce_subscription_item_list

> models::PaginatedCommerceSubscriptionItemResponse get_commerce_subscription_item_list(paginated, limit, offset, status, payer_type, plan_id, include_free, query, user_id, organization_id)
List all subscription items

Returns a list of all subscription items for the instance. The subscription items are returned sorted by creation date, with the newest appearing first. This includes subscriptions for both users and organizations. Pagination is supported.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**paginated** | Option<**bool**> | Whether to paginate the results. If true, the results will be paginated. If false, the results will not be paginated. |  |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]
**status** | Option<**String**> | Filter subscription items by status |  |
**payer_type** | Option<**String**> | Filter subscription items by payer type |  |
**plan_id** | Option<**String**> | Filter subscription items by plan ID |  |
**include_free** | Option<**bool**> | Whether to include free plan subscription items |  |[default to false]
**query** | Option<**String**> | Search query to filter subscription items by email, user first name, user last name, or organization name. Supports partial matching. |  |
**user_id** | Option<**String**> | Filter subscription items by user ID |  |
**organization_id** | Option<**String**> | Filter subscription items by organization ID |  |

### Return type

[**models::PaginatedCommerceSubscriptionItemResponse**](PaginatedCommerceSubscriptionItemResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

