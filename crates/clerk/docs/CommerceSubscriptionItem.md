# CommerceSubscriptionItem

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**object** | **String** | String representing the object's type. Objects of the same type share the same value. | 
**id** | **String** | Unique identifier for the subscription item. | 
**instance_id** | **String** | Unique identifier for the Clerk instance. | 
**status** | **String** | Current status of the subscription item. | 
**credit** | Option<[**models::CommerceSubscriptionCreditResponse**](CommerceSubscriptionCreditResponse.md)> | Credit information (only available in PaymentAttempt events). | [optional]
**plan_id** | **String** | Unique identifier for the associated plan. | 
**plan** | Option<[**models::CommercePlan**](CommercePlan.md)> | The associated commerce plan. | 
**plan_period** | **String** | The billing period for this subscription. | 
**payment_source_id** | **String** | Unique identifier for the payment source. | 
**payment_source** | Option<[**models::CommercePaymentSourceResponse**](CommercePaymentSourceResponse.md)> | The payment source associated with this subscription. | [optional]
**lifetime_paid** | Option<[**models::CommerceMoneyResponse**](CommerceMoneyResponse.md)> | Total amount paid over the lifetime of this subscription. | [optional]
**amount** | Option<[**models::CommerceMoneyResponse**](CommerceMoneyResponse.md)> | Current amount for this subscription. | [optional]
**next_invoice** | Option<[**models::CommerceSubscriptionItemNextPaymentResponse**](CommerceSubscriptionItemNextPaymentResponse.md)> | Information about the next invoice. | [optional]
**next_payment** | Option<[**models::CommerceSubscriptionItemNextPaymentResponse**](CommerceSubscriptionItemNextPaymentResponse.md)> | Information about the next payment. | [optional]
**payer_id** | **String** | Unique identifier for the payer. | 
**payer** | Option<[**models::CommercePayerResponse**](CommercePayerResponse.md)> | The payer associated with this subscription. | [optional]
**is_free_trial** | **bool** | Whether this subscription is currently on a free trial. | 
**period_start** | Option<**i64**> | Unix timestamp (in milliseconds) when the current period started. | [optional]
**period_end** | Option<**i64**> | Unix timestamp (in milliseconds) when the current period ends. | [optional]
**proration_date** | **String** | Date used for proration calculations. | 
**canceled_at** | Option<**i64**> | Unix timestamp (in milliseconds) when the subscription was canceled. | [optional]
**past_due_at** | Option<**i64**> | Unix timestamp (in milliseconds) when the subscription became past due. | [optional]
**ended_at** | Option<**i64**> | Unix timestamp (in milliseconds) when the subscription ended. | [optional]
**created_at** | **i64** | Unix timestamp (in milliseconds) when the subscription was created. | 
**updated_at** | **i64** | Unix timestamp (in milliseconds) when the subscription was last updated. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


