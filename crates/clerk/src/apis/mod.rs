use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseContent<T> {
    pub status: reqwest::StatusCode,
    pub content: String,
    pub entity: Option<T>,
}

#[derive(Debug)]
pub enum Error<T> {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ResponseContent<T>),
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            Error::Reqwest(e) => ("reqwest", e.to_string()),
            Error::Serde(e) => ("serde", e.to_string()),
            Error::Io(e) => ("IO", e.to_string()),
            Error::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl<T: fmt::Debug> error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::Reqwest(e) => e,
            Error::Serde(e) => e,
            Error::Io(e) => e,
            Error::ResponseError(_) => return None,
        })
    }
}

impl<T> From<reqwest::Error> for Error<T> {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl<T> From<serde_json::Error> for Error<T> {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl<T> From<std::io::Error> for Error<T> {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                }
                serde_json::Value::String(s) => {
                    params.push((format!("{}[{}]", prefix, key), s.clone()))
                }
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

/// Internal use only
/// A content type supported by this client.
#[allow(dead_code)]
enum ContentType {
    Json,
    Text,
    Unsupported(String),
}

impl From<&str> for ContentType {
    fn from(content_type: &str) -> Self {
        if content_type.starts_with("application") && content_type.contains("json") {
            return Self::Json;
        } else if content_type.starts_with("text/plain") {
            return Self::Text;
        } else {
            return Self::Unsupported(content_type.to_string());
        }
    }
}

pub mod accountless_applications_api;
pub mod actor_tokens_api;
pub mod allow_list_block_list_api;
pub mod aws_credentials_api;
pub mod beta_features_api;
pub mod billing_api;
pub mod clients_api;
pub mod commerce_api;
pub mod domains_api;
pub mod email_addresses_api;
pub mod email_sms_templates_api;
pub mod instance_settings_api;
pub mod invitations_api;
pub mod jwks_api;
pub mod jwt_templates_api;
pub mod m2_m_tokens_api;
pub mod machines_api;
pub mod miscellaneous_api;
pub mod o_auth_access_tokens_api;
pub mod o_auth_applications_api;
pub mod organization_domains_api;
pub mod organization_invitations_api;
pub mod organization_memberships_api;
pub mod organizations_api;
pub mod phone_numbers_api;
pub mod proxy_checks_api;
pub mod redirect_urls_api;
pub mod saml_connections_api;
pub mod sessions_api;
pub mod sign_in_tokens_api;
pub mod sign_ups_api;
pub mod testing_tokens_api;
pub mod users_api;
pub mod waitlist_entries_api;
pub mod webhooks_api;

pub mod configuration;

use std::sync::Arc;

pub trait Api {
    fn aws_credentials_api(&self) -> &dyn aws_credentials_api::AwsCredentialsApi;
    fn accountless_applications_api(
        &self,
    ) -> &dyn accountless_applications_api::AccountlessApplicationsApi;
    fn actor_tokens_api(&self) -> &dyn actor_tokens_api::ActorTokensApi;
    fn allow_list_block_list_api(&self) -> &dyn allow_list_block_list_api::AllowListBlockListApi;
    fn beta_features_api(&self) -> &dyn beta_features_api::BetaFeaturesApi;
    fn billing_api(&self) -> &dyn billing_api::BillingApi;
    fn clients_api(&self) -> &dyn clients_api::ClientsApi;
    fn commerce_api(&self) -> &dyn commerce_api::CommerceApi;
    fn domains_api(&self) -> &dyn domains_api::DomainsApi;
    fn email_addresses_api(&self) -> &dyn email_addresses_api::EmailAddressesApi;
    fn email_sms_templates_api(&self) -> &dyn email_sms_templates_api::EmailSmsTemplatesApi;
    fn instance_settings_api(&self) -> &dyn instance_settings_api::InstanceSettingsApi;
    fn invitations_api(&self) -> &dyn invitations_api::InvitationsApi;
    fn jwks_api(&self) -> &dyn jwks_api::JwksApi;
    fn jwt_templates_api(&self) -> &dyn jwt_templates_api::JwtTemplatesApi;
    fn m2_m_tokens_api(&self) -> &dyn m2_m_tokens_api::M2MTokensApi;
    fn machines_api(&self) -> &dyn machines_api::MachinesApi;
    fn miscellaneous_api(&self) -> &dyn miscellaneous_api::MiscellaneousApi;
    fn o_auth_access_tokens_api(&self) -> &dyn o_auth_access_tokens_api::OAuthAccessTokensApi;
    fn o_auth_applications_api(&self) -> &dyn o_auth_applications_api::OAuthApplicationsApi;
    fn organization_domains_api(&self) -> &dyn organization_domains_api::OrganizationDomainsApi;
    fn organization_invitations_api(
        &self,
    ) -> &dyn organization_invitations_api::OrganizationInvitationsApi;
    fn organization_memberships_api(
        &self,
    ) -> &dyn organization_memberships_api::OrganizationMembershipsApi;
    fn organizations_api(&self) -> &dyn organizations_api::OrganizationsApi;
    fn phone_numbers_api(&self) -> &dyn phone_numbers_api::PhoneNumbersApi;
    fn proxy_checks_api(&self) -> &dyn proxy_checks_api::ProxyChecksApi;
    fn redirect_urls_api(&self) -> &dyn redirect_urls_api::RedirectUrlsApi;
    fn saml_connections_api(&self) -> &dyn saml_connections_api::SamlConnectionsApi;
    fn sessions_api(&self) -> &dyn sessions_api::SessionsApi;
    fn sign_in_tokens_api(&self) -> &dyn sign_in_tokens_api::SignInTokensApi;
    fn sign_ups_api(&self) -> &dyn sign_ups_api::SignUpsApi;
    fn testing_tokens_api(&self) -> &dyn testing_tokens_api::TestingTokensApi;
    fn users_api(&self) -> &dyn users_api::UsersApi;
    fn waitlist_entries_api(&self) -> &dyn waitlist_entries_api::WaitlistEntriesApi;
    fn webhooks_api(&self) -> &dyn webhooks_api::WebhooksApi;
}

pub struct ApiClient {
    aws_credentials_api: Box<dyn aws_credentials_api::AwsCredentialsApi>,
    accountless_applications_api: Box<dyn accountless_applications_api::AccountlessApplicationsApi>,
    actor_tokens_api: Box<dyn actor_tokens_api::ActorTokensApi>,
    allow_list_block_list_api: Box<dyn allow_list_block_list_api::AllowListBlockListApi>,
    beta_features_api: Box<dyn beta_features_api::BetaFeaturesApi>,
    billing_api: Box<dyn billing_api::BillingApi>,
    clients_api: Box<dyn clients_api::ClientsApi>,
    commerce_api: Box<dyn commerce_api::CommerceApi>,
    domains_api: Box<dyn domains_api::DomainsApi>,
    email_addresses_api: Box<dyn email_addresses_api::EmailAddressesApi>,
    email_sms_templates_api: Box<dyn email_sms_templates_api::EmailSmsTemplatesApi>,
    instance_settings_api: Box<dyn instance_settings_api::InstanceSettingsApi>,
    invitations_api: Box<dyn invitations_api::InvitationsApi>,
    jwks_api: Box<dyn jwks_api::JwksApi>,
    jwt_templates_api: Box<dyn jwt_templates_api::JwtTemplatesApi>,
    m2_m_tokens_api: Box<dyn m2_m_tokens_api::M2MTokensApi>,
    machines_api: Box<dyn machines_api::MachinesApi>,
    miscellaneous_api: Box<dyn miscellaneous_api::MiscellaneousApi>,
    o_auth_access_tokens_api: Box<dyn o_auth_access_tokens_api::OAuthAccessTokensApi>,
    o_auth_applications_api: Box<dyn o_auth_applications_api::OAuthApplicationsApi>,
    organization_domains_api: Box<dyn organization_domains_api::OrganizationDomainsApi>,
    organization_invitations_api: Box<dyn organization_invitations_api::OrganizationInvitationsApi>,
    organization_memberships_api: Box<dyn organization_memberships_api::OrganizationMembershipsApi>,
    organizations_api: Box<dyn organizations_api::OrganizationsApi>,
    phone_numbers_api: Box<dyn phone_numbers_api::PhoneNumbersApi>,
    proxy_checks_api: Box<dyn proxy_checks_api::ProxyChecksApi>,
    redirect_urls_api: Box<dyn redirect_urls_api::RedirectUrlsApi>,
    saml_connections_api: Box<dyn saml_connections_api::SamlConnectionsApi>,
    sessions_api: Box<dyn sessions_api::SessionsApi>,
    sign_in_tokens_api: Box<dyn sign_in_tokens_api::SignInTokensApi>,
    sign_ups_api: Box<dyn sign_ups_api::SignUpsApi>,
    testing_tokens_api: Box<dyn testing_tokens_api::TestingTokensApi>,
    users_api: Box<dyn users_api::UsersApi>,
    waitlist_entries_api: Box<dyn waitlist_entries_api::WaitlistEntriesApi>,
    webhooks_api: Box<dyn webhooks_api::WebhooksApi>,
}

impl ApiClient {
    pub fn new(configuration: Arc<configuration::Configuration>) -> Self {
        Self {
            aws_credentials_api: Box::new(aws_credentials_api::AwsCredentialsApiClient::new(
                configuration.clone(),
            )),
            accountless_applications_api: Box::new(
                accountless_applications_api::AccountlessApplicationsApiClient::new(
                    configuration.clone(),
                ),
            ),
            actor_tokens_api: Box::new(actor_tokens_api::ActorTokensApiClient::new(
                configuration.clone(),
            )),
            allow_list_block_list_api: Box::new(
                allow_list_block_list_api::AllowListBlockListApiClient::new(configuration.clone()),
            ),
            beta_features_api: Box::new(beta_features_api::BetaFeaturesApiClient::new(
                configuration.clone(),
            )),
            billing_api: Box::new(billing_api::BillingApiClient::new(configuration.clone())),
            clients_api: Box::new(clients_api::ClientsApiClient::new(configuration.clone())),
            commerce_api: Box::new(commerce_api::CommerceApiClient::new(configuration.clone())),
            domains_api: Box::new(domains_api::DomainsApiClient::new(configuration.clone())),
            email_addresses_api: Box::new(email_addresses_api::EmailAddressesApiClient::new(
                configuration.clone(),
            )),
            email_sms_templates_api: Box::new(
                email_sms_templates_api::EmailSmsTemplatesApiClient::new(configuration.clone()),
            ),
            instance_settings_api: Box::new(instance_settings_api::InstanceSettingsApiClient::new(
                configuration.clone(),
            )),
            invitations_api: Box::new(invitations_api::InvitationsApiClient::new(
                configuration.clone(),
            )),
            jwks_api: Box::new(jwks_api::JwksApiClient::new(configuration.clone())),
            jwt_templates_api: Box::new(jwt_templates_api::JwtTemplatesApiClient::new(
                configuration.clone(),
            )),
            m2_m_tokens_api: Box::new(m2_m_tokens_api::M2MTokensApiClient::new(
                configuration.clone(),
            )),
            machines_api: Box::new(machines_api::MachinesApiClient::new(configuration.clone())),
            miscellaneous_api: Box::new(miscellaneous_api::MiscellaneousApiClient::new(
                configuration.clone(),
            )),
            o_auth_access_tokens_api: Box::new(
                o_auth_access_tokens_api::OAuthAccessTokensApiClient::new(configuration.clone()),
            ),
            o_auth_applications_api: Box::new(
                o_auth_applications_api::OAuthApplicationsApiClient::new(configuration.clone()),
            ),
            organization_domains_api: Box::new(
                organization_domains_api::OrganizationDomainsApiClient::new(configuration.clone()),
            ),
            organization_invitations_api: Box::new(
                organization_invitations_api::OrganizationInvitationsApiClient::new(
                    configuration.clone(),
                ),
            ),
            organization_memberships_api: Box::new(
                organization_memberships_api::OrganizationMembershipsApiClient::new(
                    configuration.clone(),
                ),
            ),
            organizations_api: Box::new(organizations_api::OrganizationsApiClient::new(
                configuration.clone(),
            )),
            phone_numbers_api: Box::new(phone_numbers_api::PhoneNumbersApiClient::new(
                configuration.clone(),
            )),
            proxy_checks_api: Box::new(proxy_checks_api::ProxyChecksApiClient::new(
                configuration.clone(),
            )),
            redirect_urls_api: Box::new(redirect_urls_api::RedirectUrlsApiClient::new(
                configuration.clone(),
            )),
            saml_connections_api: Box::new(saml_connections_api::SamlConnectionsApiClient::new(
                configuration.clone(),
            )),
            sessions_api: Box::new(sessions_api::SessionsApiClient::new(configuration.clone())),
            sign_in_tokens_api: Box::new(sign_in_tokens_api::SignInTokensApiClient::new(
                configuration.clone(),
            )),
            sign_ups_api: Box::new(sign_ups_api::SignUpsApiClient::new(configuration.clone())),
            testing_tokens_api: Box::new(testing_tokens_api::TestingTokensApiClient::new(
                configuration.clone(),
            )),
            users_api: Box::new(users_api::UsersApiClient::new(configuration.clone())),
            waitlist_entries_api: Box::new(waitlist_entries_api::WaitlistEntriesApiClient::new(
                configuration.clone(),
            )),
            webhooks_api: Box::new(webhooks_api::WebhooksApiClient::new(configuration.clone())),
        }
    }
}

impl Api for ApiClient {
    fn aws_credentials_api(&self) -> &dyn aws_credentials_api::AwsCredentialsApi {
        self.aws_credentials_api.as_ref()
    }
    fn accountless_applications_api(
        &self,
    ) -> &dyn accountless_applications_api::AccountlessApplicationsApi {
        self.accountless_applications_api.as_ref()
    }
    fn actor_tokens_api(&self) -> &dyn actor_tokens_api::ActorTokensApi {
        self.actor_tokens_api.as_ref()
    }
    fn allow_list_block_list_api(&self) -> &dyn allow_list_block_list_api::AllowListBlockListApi {
        self.allow_list_block_list_api.as_ref()
    }
    fn beta_features_api(&self) -> &dyn beta_features_api::BetaFeaturesApi {
        self.beta_features_api.as_ref()
    }
    fn billing_api(&self) -> &dyn billing_api::BillingApi {
        self.billing_api.as_ref()
    }
    fn clients_api(&self) -> &dyn clients_api::ClientsApi {
        self.clients_api.as_ref()
    }
    fn commerce_api(&self) -> &dyn commerce_api::CommerceApi {
        self.commerce_api.as_ref()
    }
    fn domains_api(&self) -> &dyn domains_api::DomainsApi {
        self.domains_api.as_ref()
    }
    fn email_addresses_api(&self) -> &dyn email_addresses_api::EmailAddressesApi {
        self.email_addresses_api.as_ref()
    }
    fn email_sms_templates_api(&self) -> &dyn email_sms_templates_api::EmailSmsTemplatesApi {
        self.email_sms_templates_api.as_ref()
    }
    fn instance_settings_api(&self) -> &dyn instance_settings_api::InstanceSettingsApi {
        self.instance_settings_api.as_ref()
    }
    fn invitations_api(&self) -> &dyn invitations_api::InvitationsApi {
        self.invitations_api.as_ref()
    }
    fn jwks_api(&self) -> &dyn jwks_api::JwksApi {
        self.jwks_api.as_ref()
    }
    fn jwt_templates_api(&self) -> &dyn jwt_templates_api::JwtTemplatesApi {
        self.jwt_templates_api.as_ref()
    }
    fn m2_m_tokens_api(&self) -> &dyn m2_m_tokens_api::M2MTokensApi {
        self.m2_m_tokens_api.as_ref()
    }
    fn machines_api(&self) -> &dyn machines_api::MachinesApi {
        self.machines_api.as_ref()
    }
    fn miscellaneous_api(&self) -> &dyn miscellaneous_api::MiscellaneousApi {
        self.miscellaneous_api.as_ref()
    }
    fn o_auth_access_tokens_api(&self) -> &dyn o_auth_access_tokens_api::OAuthAccessTokensApi {
        self.o_auth_access_tokens_api.as_ref()
    }
    fn o_auth_applications_api(&self) -> &dyn o_auth_applications_api::OAuthApplicationsApi {
        self.o_auth_applications_api.as_ref()
    }
    fn organization_domains_api(&self) -> &dyn organization_domains_api::OrganizationDomainsApi {
        self.organization_domains_api.as_ref()
    }
    fn organization_invitations_api(
        &self,
    ) -> &dyn organization_invitations_api::OrganizationInvitationsApi {
        self.organization_invitations_api.as_ref()
    }
    fn organization_memberships_api(
        &self,
    ) -> &dyn organization_memberships_api::OrganizationMembershipsApi {
        self.organization_memberships_api.as_ref()
    }
    fn organizations_api(&self) -> &dyn organizations_api::OrganizationsApi {
        self.organizations_api.as_ref()
    }
    fn phone_numbers_api(&self) -> &dyn phone_numbers_api::PhoneNumbersApi {
        self.phone_numbers_api.as_ref()
    }
    fn proxy_checks_api(&self) -> &dyn proxy_checks_api::ProxyChecksApi {
        self.proxy_checks_api.as_ref()
    }
    fn redirect_urls_api(&self) -> &dyn redirect_urls_api::RedirectUrlsApi {
        self.redirect_urls_api.as_ref()
    }
    fn saml_connections_api(&self) -> &dyn saml_connections_api::SamlConnectionsApi {
        self.saml_connections_api.as_ref()
    }
    fn sessions_api(&self) -> &dyn sessions_api::SessionsApi {
        self.sessions_api.as_ref()
    }
    fn sign_in_tokens_api(&self) -> &dyn sign_in_tokens_api::SignInTokensApi {
        self.sign_in_tokens_api.as_ref()
    }
    fn sign_ups_api(&self) -> &dyn sign_ups_api::SignUpsApi {
        self.sign_ups_api.as_ref()
    }
    fn testing_tokens_api(&self) -> &dyn testing_tokens_api::TestingTokensApi {
        self.testing_tokens_api.as_ref()
    }
    fn users_api(&self) -> &dyn users_api::UsersApi {
        self.users_api.as_ref()
    }
    fn waitlist_entries_api(&self) -> &dyn waitlist_entries_api::WaitlistEntriesApi {
        self.waitlist_entries_api.as_ref()
    }
    fn webhooks_api(&self) -> &dyn webhooks_api::WebhooksApi {
        self.webhooks_api.as_ref()
    }
}
