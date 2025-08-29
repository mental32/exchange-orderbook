# \UsersApi

All URIs are relative to *https://api.clerk.com/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**ban_user**](UsersApi.md#ban_user) | **POST** /users/{user_id}/ban | Ban a user
[**create_user**](UsersApi.md#create_user) | **POST** /users | Create a new user
[**delete_backup_code**](UsersApi.md#delete_backup_code) | **DELETE** /users/{user_id}/backup_code | Disable all user's Backup codes
[**delete_external_account**](UsersApi.md#delete_external_account) | **DELETE** /users/{user_id}/external_accounts/{external_account_id} | Delete External Account
[**delete_totp**](UsersApi.md#delete_totp) | **DELETE** /users/{user_id}/totp | Delete all the user's TOTPs
[**delete_user**](UsersApi.md#delete_user) | **DELETE** /users/{user_id} | Delete a user
[**delete_user_profile_image**](UsersApi.md#delete_user_profile_image) | **DELETE** /users/{user_id}/profile_image | Delete user profile image
[**disable_mfa**](UsersApi.md#disable_mfa) | **DELETE** /users/{user_id}/mfa | Disable a user's MFA methods
[**get_o_auth_access_token**](UsersApi.md#get_o_auth_access_token) | **GET** /users/{user_id}/oauth_access_tokens/{provider} | Retrieve the OAuth access token of a user
[**get_user**](UsersApi.md#get_user) | **GET** /users/{user_id} | Retrieve a user
[**get_user_billing_subscription**](UsersApi.md#get_user_billing_subscription) | **GET** /users/{user_id}/billing/subscription | Retrieve a user's billing subscription
[**get_user_list**](UsersApi.md#get_user_list) | **GET** /users | List all users
[**get_users_count**](UsersApi.md#get_users_count) | **GET** /users/count | Count users
[**lock_user**](UsersApi.md#lock_user) | **POST** /users/{user_id}/lock | Lock a user
[**set_user_profile_image**](UsersApi.md#set_user_profile_image) | **POST** /users/{user_id}/profile_image | Set user profile image
[**unban_user**](UsersApi.md#unban_user) | **POST** /users/{user_id}/unban | Unban a user
[**unlock_user**](UsersApi.md#unlock_user) | **POST** /users/{user_id}/unlock | Unlock a user
[**update_user**](UsersApi.md#update_user) | **PATCH** /users/{user_id} | Update a user
[**update_user_metadata**](UsersApi.md#update_user_metadata) | **PATCH** /users/{user_id}/metadata | Merge and update a user's metadata
[**user_passkey_delete**](UsersApi.md#user_passkey_delete) | **DELETE** /users/{user_id}/passkeys/{passkey_identification_id} | Delete a user passkey
[**user_web3_wallet_delete**](UsersApi.md#user_web3_wallet_delete) | **DELETE** /users/{user_id}/web3_wallets/{web3_wallet_identification_id} | Delete a user web3 wallet
[**users_ban**](UsersApi.md#users_ban) | **POST** /users/ban | Ban multiple users
[**users_get_organization_invitations**](UsersApi.md#users_get_organization_invitations) | **GET** /users/{user_id}/organization_invitations | Retrieve all invitations for a user
[**users_get_organization_memberships**](UsersApi.md#users_get_organization_memberships) | **GET** /users/{user_id}/organization_memberships | Retrieve all memberships for a user
[**users_unban**](UsersApi.md#users_unban) | **POST** /users/unban | Unban multiple users
[**verify_password**](UsersApi.md#verify_password) | **POST** /users/{user_id}/verify_password | Verify the password of a user
[**verify_totp**](UsersApi.md#verify_totp) | **POST** /users/{user_id}/verify_totp | Verify a TOTP or backup code for a user



## ban_user

> models::User ban_user(user_id)
Ban a user

Marks the given user as banned, which means that all their sessions are revoked and they are not allowed to sign in again.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to ban | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_user

> models::User create_user(create_user_request)
Create a new user

Creates a new user. Your user management settings determine how you should setup your user model.  Any email address and phone number created using this method will be marked as verified.  Note: If you are performing a migration, check out our guide on [zero downtime migrations](https://clerk.com/docs/deployments/migrate-overview).  The following rate limit rules apply to this endpoint: 1000 requests per 10 seconds for production instances and 100 requests per 10 seconds for development instances

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_user_request** | [**CreateUserRequest**](CreateUserRequest.md) |  | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_backup_code

> models::DisableMfa200Response delete_backup_code(user_id)
Disable all user's Backup codes

Disable all of a user's backup codes.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose backup codes are to be deleted. | [required] |

### Return type

[**models::DisableMfa200Response**](DisableMFA_200_response.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_external_account

> models::DeletedObject delete_external_account(user_id, external_account_id)
Delete External Account

Delete an external account by ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user's external account | [required] |
**external_account_id** | **String** | The ID of the external account to delete | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_totp

> models::DisableMfa200Response delete_totp(user_id)
Delete all the user's TOTPs

Deletes all of the user's TOTPs.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose TOTPs are to be deleted | [required] |

### Return type

[**models::DisableMfa200Response**](DisableMFA_200_response.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_user

> models::DeletedObject delete_user(user_id)
Delete a user

Delete the specified user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to delete | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_user_profile_image

> models::User delete_user_profile_image(user_id)
Delete user profile image

Delete a user's profile image

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to delete the profile image for | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## disable_mfa

> models::DisableMfa200Response disable_mfa(user_id)
Disable a user's MFA methods

Disable all of a user's MFA methods (e.g. OTP sent via SMS, TOTP on their authenticator app) at once.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose MFA methods are to be disabled | [required] |

### Return type

[**models::DisableMfa200Response**](DisableMFA_200_response.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_o_auth_access_token

> Vec<models::OAuthAccessTokenInner> get_o_auth_access_token(user_id, provider, paginated, limit, offset)
Retrieve the OAuth access token of a user

Fetch the corresponding OAuth access token for a user that has previously authenticated with a particular OAuth provider. For OAuth 2.0, if the access token has expired and we have a corresponding refresh token, the access token will be refreshed transparently the new one will be returned.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user for which to retrieve the OAuth access token | [required] |
**provider** | **String** | The ID of the OAuth provider (e.g. `oauth_google`) | [required] |
**paginated** | Option<**bool**> | Whether to paginate the results. If true, the results will be paginated. If false, the results will not be paginated. |  |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]

### Return type

[**Vec<models::OAuthAccessTokenInner>**](OAuthAccessToken_inner.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_user

> models::User get_user(user_id)
Retrieve a user

Retrieve the details of a user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to retrieve | [required] |

### Return type

[**models::User**](User.md)

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


## get_user_list

> Vec<models::User> get_user_list(email_address, phone_number, external_id, username, web3_wallet, user_id, organization_id, query, email_address_query, phone_number_query, username_query, name_query, banned, last_active_at_before, last_active_at_after, last_active_at_since, created_at_before, created_at_after, limit, offset, order_by)
List all users

Returns a list of all users. The users are returned sorted by creation date, with the newest users appearing first.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**email_address** | Option<[**Vec<String>**](String.md)> | Returns users with the specified email addresses. Accepts up to 100 email addresses. Any email addresses not found are ignored. |  |
**phone_number** | Option<[**Vec<String>**](String.md)> | Returns users with the specified phone numbers. Accepts up to 100 phone numbers. Any phone numbers not found are ignored. |  |
**external_id** | Option<[**Vec<String>**](String.md)> | Returns users with the specified external ids. For each external id, the `+` and `-` can be prepended to the id, which denote whether the respective external id should be included or excluded from the result set. Accepts up to 100 external ids. Any external ids not found are ignored. |  |
**username** | Option<[**Vec<String>**](String.md)> | Returns users with the specified usernames. Accepts up to 100 usernames. Any usernames not found are ignored. |  |
**web3_wallet** | Option<[**Vec<String>**](String.md)> | Returns users with the specified web3 wallet addresses. Accepts up to 100 web3 wallet addresses. Any web3 wallet addressed not found are ignored. |  |
**user_id** | Option<[**Vec<String>**](String.md)> | Returns users with the user ids specified. For each user id, the `+` and `-` can be prepended to the id, which denote whether the respective user id should be included or excluded from the result set. Accepts up to 100 user ids. Any user ids not found are ignored. |  |
**organization_id** | Option<[**Vec<String>**](String.md)> | Returns users that have memberships to the given organizations. For each organization id, the `+` and `-` can be prepended to the id, which denote whether the respective organization should be included or excluded from the result set. Accepts up to 100 organization ids. |  |
**query** | Option<**String**> | Returns users that match the given query. For possible matches, we check the email addresses, phone numbers, usernames, web3 wallets, user ids, first and last names. The query value doesn't need to match the exact value you are looking for, it is capable of partial matches as well. |  |
**email_address_query** | Option<**String**> | Returns users with emails that match the given query, via case-insensitive partial match. For example, `email_address_query=ello` will match a user with the email `HELLO@example.com`. |  |
**phone_number_query** | Option<**String**> | Returns users with phone numbers that match the given query, via case-insensitive partial match. For example, `phone_number_query=555` will match a user with the phone number `+1555xxxxxxx`. |  |
**username_query** | Option<**String**> | Returns users with usernames that match the given query, via case-insensitive partial match. For example, `username_query=CoolUser` will match a user with the username `SomeCoolUser`. |  |
**name_query** | Option<**String**> | Returns users with names that match the given query, via case-insensitive partial match. |  |
**banned** | Option<**bool**> | Returns users which are either banned (`banned=true`) or not banned (`banned=false`). |  |
**last_active_at_before** | Option<**i32**> | Returns users whose last session activity was before the given date (with millisecond precision). Example: use 1700690400000 to retrieve users whose last session activity was before 2023-11-23. |  |
**last_active_at_after** | Option<**i32**> | Returns users whose last session activity was after the given date (with millisecond precision). Example: use 1700690400000 to retrieve users whose last session activity was after 2023-11-23. |  |
**last_active_at_since** | Option<**i32**> | Returns users that had session activity since the given date. Example: use 1700690400000 to retrieve users that had session activity from 2023-11-23 until the current day. Deprecated in favor of `last_active_at_after`. |  |
**created_at_before** | Option<**i32**> | Returns users who have been created before the given date (with millisecond precision). Example: use 1730160000000 to retrieve users who have been created before 2024-10-29. |  |
**created_at_after** | Option<**i32**> | Returns users who have been created after the given date (with millisecond precision). Example: use 1730160000000 to retrieve users who have been created after 2024-10-29. |  |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]
**order_by** | Option<**String**> | Allows to return users in a particular order. At the moment, you can order the returned users by their `created_at`,`updated_at`,`email_address`,`web3wallet`,`first_name`,`last_name`,`phone_number`,`username`,`last_active_at`,`last_sign_in_at`. In order to specify the direction, you can use the `+/-` symbols prepended in the property to order by. For example, if you want users to be returned in descending order according to their `created_at` property, you can use `-created_at`. If you don't use `+` or `-`, then `+` is implied. We only support one `order_by` parameter, and if multiple `order_by` parameters are provided, we will only keep the first one. For example, if you pass `order_by=username&order_by=created_at`, we will consider only the first `order_by` parameter, which is `username`. The `created_at` parameter will be ignored in this case. |  |[default to -created_at]

### Return type

[**Vec<models::User>**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_users_count

> models::TotalCount get_users_count(email_address, phone_number, external_id, username, web3_wallet, user_id, organization_id, query, email_address_query, phone_number_query, username_query, name_query, banned, last_active_at_before, last_active_at_after, last_active_at_since, created_at_before, created_at_after)
Count users

Returns a total count of all users that match the given filtering criteria.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**email_address** | Option<[**Vec<String>**](String.md)> | Counts users with the specified email addresses. Accepts up to 100 email addresses. Any email addresses not found are ignored. |  |
**phone_number** | Option<[**Vec<String>**](String.md)> | Counts users with the specified phone numbers. Accepts up to 100 phone numbers. Any phone numbers not found are ignored. |  |
**external_id** | Option<[**Vec<String>**](String.md)> | Counts users with the specified external ids. Accepts up to 100 external ids. Any external ids not found are ignored. |  |
**username** | Option<[**Vec<String>**](String.md)> | Counts users with the specified usernames. Accepts up to 100 usernames. Any usernames not found are ignored. |  |
**web3_wallet** | Option<[**Vec<String>**](String.md)> | Counts users with the specified web3 wallet addresses. Accepts up to 100 web3 wallet addresses. Any web3 wallet addressed not found are ignored. |  |
**user_id** | Option<[**Vec<String>**](String.md)> | Counts users with the user ids specified. Accepts up to 100 user ids. Any user ids not found are ignored. |  |
**organization_id** | Option<[**Vec<String>**](String.md)> | Returns users that have memberships to the given organizations. For each organization id, the `+` and `-` can be prepended to the id, which denote whether the respective organization should be included or excluded from the result set. Accepts up to 100 organization ids. |  |
**query** | Option<**String**> | Counts users that match the given query. For possible matches, we check the email addresses, phone numbers, usernames, web3 wallets, user ids, first and last names. The query value doesn't need to match the exact value you are looking for, it is capable of partial matches as well. |  |
**email_address_query** | Option<**String**> | Counts users with emails that match the given query, via case-insensitive partial match. For example, `email_address_query=ello` will match a user with the email `HELLO@example.com`, and will be included in the resulting count. |  |
**phone_number_query** | Option<**String**> | Counts users with phone numbers that match the given query, via case-insensitive partial match. For example, `phone_number_query=555` will match a user with the phone number `+1555xxxxxxx`, and will be included in the resulting count. |  |
**username_query** | Option<**String**> | Counts users with usernames that match the given query, via case-insensitive partial match. For example, `username_query=CoolUser` will match a user with the username `SomeCoolUser`, and will be included in the resulting count. |  |
**name_query** | Option<**String**> | Returns users with names that match the given query, via case-insensitive partial match. |  |
**banned** | Option<**bool**> | Counts users which are either banned (`banned=true`) or not banned (`banned=false`). |  |
**last_active_at_before** | Option<**i32**> | Returns users whose last session activity was before the given date (with millisecond precision). Example: use 1700690400000 to retrieve users whose last session activity was before 2023-11-23. |  |
**last_active_at_after** | Option<**i32**> | Returns users whose last session activity was after the given date (with millisecond precision). Example: use 1700690400000 to retrieve users whose last session activity was after 2023-11-23. |  |
**last_active_at_since** | Option<**i32**> | Returns users that had session activity since the given date. Example: use 1700690400000 to retrieve users that had session activity from 2023-11-23 until the current day. Deprecated in favor of `last_active_at_after`. |  |
**created_at_before** | Option<**i32**> | Returns users who have been created before the given date (with millisecond precision). Example: use 1730160000000 to retrieve users who have been created before 2024-10-29. |  |
**created_at_after** | Option<**i32**> | Returns users who have been created after the given date (with millisecond precision). Example: use 1730160000000 to retrieve users who have been created after 2024-10-29. |  |

### Return type

[**models::TotalCount**](TotalCount.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## lock_user

> models::User lock_user(user_id)
Lock a user

Marks the given user as locked, which means they are not allowed to sign in again until the lock expires. Lock duration can be configured in the instance's restrictions settings.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to lock | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## set_user_profile_image

> models::User set_user_profile_image(user_id, file)
Set user profile image

Update a user's profile image

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to update the profile image for | [required] |
**file** | Option<**std::path::PathBuf**> |  |  |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: multipart/form-data
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## unban_user

> models::User unban_user(user_id)
Unban a user

Removes the ban mark from the given user.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to unban | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## unlock_user

> models::User unlock_user(user_id)
Unlock a user

Removes the lock from the given user.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to unlock | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_user

> models::User update_user(user_id, update_user_request)
Update a user

Update a user's attributes.  You can set the user's primary contact identifiers (email address and phone numbers) by updating the `primary_email_address_id` and `primary_phone_number_id` attributes respectively. Both IDs should correspond to verified identifications that belong to the user.  You can remove a user's username by setting the username attribute to null or the blank string \"\". This is a destructive action; the identification will be deleted forever. Usernames can be removed only if they are optional in your instance settings and there's at least one other identifier which can be used for authentication.  This endpoint allows changing a user's password. When passing the `password` parameter directly you have two further options. You can ignore the password policy checks for your instance by setting the `skip_password_checks` parameter to `true`. You can also choose to sign the user out of all their active sessions on any device once the password is updated. Just set `sign_out_of_other_sessions` to `true`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user to update | [required] |
**update_user_request** | [**UpdateUserRequest**](UpdateUserRequest.md) |  | [required] |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_user_metadata

> models::User update_user_metadata(user_id, update_user_metadata_request)
Merge and update a user's metadata

Update a user's metadata attributes by merging existing values with the provided parameters.  This endpoint behaves differently than the *Update a user* endpoint. Metadata values will not be replaced entirely. Instead, a deep merge will be performed. Deep means that any nested JSON objects will be merged as well.  You can remove metadata keys at any level by setting their value to `null`.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose metadata will be updated and merged | [required] |
**update_user_metadata_request** | Option<[**UpdateUserMetadataRequest**](UpdateUserMetadataRequest.md)> |  |  |

### Return type

[**models::User**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## user_passkey_delete

> models::DeletedObject user_passkey_delete(user_id, passkey_identification_id)
Delete a user passkey

Delete the passkey identification for a given user and notify them through email.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user that owns the passkey identity | [required] |
**passkey_identification_id** | **String** | The ID of the passkey identity to be deleted | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## user_web3_wallet_delete

> models::DeletedObject user_web3_wallet_delete(user_id, web3_wallet_identification_id)
Delete a user web3 wallet

Delete the web3 wallet identification for a given user.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user that owns the web3 wallet | [required] |
**web3_wallet_identification_id** | **String** | The ID of the web3 wallet identity to be deleted | [required] |

### Return type

[**models::DeletedObject**](DeletedObject.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_ban

> Vec<models::User> users_ban(users_ban_request)
Ban multiple users

Marks multiple users as banned, which means that all their sessions are revoked and they are not allowed to sign in again.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**users_ban_request** | [**UsersBanRequest**](UsersBanRequest.md) |  | [required] |

### Return type

[**Vec<models::User>**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_get_organization_invitations

> models::OrganizationInvitationsWithPublicOrganizationData users_get_organization_invitations(user_id, limit, offset, status)
Retrieve all invitations for a user

Retrieve a paginated list of the user's organization invitations

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose organization invitations we want to retrieve | [required] |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]
**status** | Option<**String**> | Filter organization invitations based on their status |  |

### Return type

[**models::OrganizationInvitationsWithPublicOrganizationData**](OrganizationInvitationsWithPublicOrganizationData.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_get_organization_memberships

> models::OrganizationMemberships users_get_organization_memberships(user_id, limit, offset)
Retrieve all memberships for a user

Retrieve a paginated list of the user's organization memberships

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user whose organization memberships we want to retrieve | [required] |
**limit** | Option<**i32**> | Applies a limit to the number of results returned. Can be used for paginating the results together with `offset`. |  |[default to 10]
**offset** | Option<**i32**> | Skip the first `offset` results when paginating. Needs to be an integer greater or equal to zero. To be used in conjunction with `limit`. |  |[default to 0]

### Return type

[**models::OrganizationMemberships**](OrganizationMemberships.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## users_unban

> Vec<models::User> users_unban(users_unban_request)
Unban multiple users

Removes the ban mark from multiple users.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**users_unban_request** | [**UsersUnbanRequest**](UsersUnbanRequest.md) |  | [required] |

### Return type

[**Vec<models::User>**](User.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## verify_password

> models::VerifyPassword200Response verify_password(user_id, verify_password_request)
Verify the password of a user

Check that the user's password matches the supplied input. Useful for custom auth flows and re-verification.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user for whom to verify the password | [required] |
**verify_password_request** | Option<[**VerifyPasswordRequest**](VerifyPasswordRequest.md)> |  |  |

### Return type

[**models::VerifyPassword200Response**](VerifyPassword_200_response.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## verify_totp

> models::VerifyTotp200Response verify_totp(user_id, verify_totp_request)
Verify a TOTP or backup code for a user

Verify that the provided TOTP or backup code is valid for the user. Verifying a backup code will result it in being consumed (i.e. it will become invalid). Useful for custom auth flows and re-verification.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | The ID of the user for whom to verify the TOTP | [required] |
**verify_totp_request** | Option<[**VerifyTotpRequest**](VerifyTotpRequest.md)> |  |  |

### Return type

[**models::VerifyTotp200Response**](VerifyTOTP_200_response.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

