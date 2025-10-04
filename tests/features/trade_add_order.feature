@e2e @trade_add
Feature: Place orders through the trading HTTP API
  As an integrator
  I want to place trade orders via the HTTP API
  So that the matching engine accepts orders and returns an order identifier or appropriate error codes

  Background:
    Given the exchange service is running
    And the trading API is reachable at "/trade"

  # Happy path: place a limit buy order for an enabled asset
  Scenario: Successfully place a limit buy order for an enabled asset
    Given the asset "BTC-USD" is enabled on the exchange
    When I POST "/trade/BTC-USD/order" with body:
      """
      {
        "side": "buy",
        "order_type": "limit",
        "quantity": "1",
        "price": "100",
        "time_in_force": "good_til_canceled",
        "stp": "co"
      }
      """
    Then the response status code is 200
    And the response JSON contains an "order_uuid" field

  # Negative: asset not enabled
  Scenario: Placing an order for a disabled asset returns 404
    Given the asset "FOO-USD" is not enabled on the exchange
    When I POST "/trade/FOO-USD/order" with body:
      """
      {
        "side": "buy",
        "order_type": "limit",
        "quantity": "1",
        "price": "1",
        "time_in_force": "good_til_canceled",
        "stp": "dc"
      }
      """
    Then the response status code is 404
    And the response body contains an error message

  # Business rule: IOC with insufficient liquidity should not create resting orders
  Scenario: Placing an IOC order with insufficient liquidity returns an error
    Given the orderbook for "BTC-USD" has no resting sell orders at or below price "1"
    When I POST "/trade/BTC-USD/order" with body:
      """
      {
        "side": "buy",
        "order_type": "limit",
        "quantity": "10",
        "price": "1",
        "time_in_force": "immediate_or_cancel",
        "stp": "dc"
      }
      """
    Then the response status code is in [400, 422, 500]
    And the response body contains an error message indicating insufficient liquidity

  # Edge: partial fill then resting order for GTC
  Scenario: Partial fill on GTC leaves remaining quantity as a resting order
    Given the orderbook for "BTC-USD" has a resting sell order at price "100" with quantity "1"
    When I POST "/trade/BTC-USD/order" with body:
      """
      {
        "side": "buy",
        "order_type": "limit",
        "quantity": "2",
        "price": "100",
        "time_in_force": "good_til_canceled",
        "stp": "co"
      }
      """
    Then the response status code is 200
    And the response JSON contains "order_uuid"
    And the response JSON indicates "quantity_filled" is "1"
    And the orderbook for "BTC-USD" contains a resting order for the remaining quantity "1" at price "100"

  # Validation: malformed request
  Scenario: Malformed request returns 400
    When I POST "/trade/BTC-USD/order" with body:
      """
      { "side": "buy", "order_type": "limit", "quantity": "not-a-number" }
      """
    Then the response status code is 400
    And the response body contains a validation error message

  # Authorization / session handling (happy path assumption: middleware should validate)
  Scenario: Request without valid clerk session is rejected
    Given the request has no clerk session or an invalid session token
    When I POST "/trade/BTC-USD/order" with a valid order body
    Then the response status code is 401 or 403
    And the response body contains an authentication error message
