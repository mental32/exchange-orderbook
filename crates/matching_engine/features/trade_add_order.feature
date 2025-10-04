Feature: Place orders through the trading HTTP API

  Background:
    Given the exchange is running

  @flaky
  Scenario Outline: Place a buy order
    Given a buy order of 100 units at price 100
