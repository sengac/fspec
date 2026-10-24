@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider runtime scripted dispatch
  """
  Custom-provider selection retains the upstream API model ID exposed by ProviderManager. The custom agent factory resolves an exact configured alias first, otherwise accepts only a unique upstream model ID. Rhai receives the original alias as config.model_alias and the upstream ID as config.model. Unknown or ambiguous upstream IDs fail without choosing an arbitrary alias. Exercise the real factory and local HTTP requests, including named runtime profiles, without changing manager identity semantics.
  """

  Background: User Story
    As a developer running scripted providers
    I want selected models to dispatch using their configured aliases and upstream IDs
    So that ordinary selections and named runtime profiles reach the correct model

  Scenario: Selected scripted model dispatches with its original alias
    Given a scripted provider maps smart to upstream model private/script-model
    When the selected model is used to create an agent and send a request
    Then the request uses private/script-model and the script receives alias smart

  Scenario: Named scripted runtime profile preserves dispatch identity
    Given a scripted provider profile careful selects smart mapped to private/script-model
    When the profile selection is used to create an agent and send a request
    Then the profile remains selected and the request preserves the upstream ID and original alias

  Scenario: Exact scripted aliases take priority over upstream ID matches
    Given a scripted model alias smart also matches another model's upstream ID
    When an agent is created with the exact alias smart and sends a request
    Then the request uses the model configured under smart rather than the other alias

  Scenario: Unknown scripted model identifiers are rejected
    Given a scripted provider has no alias or upstream ID matching missing/model
    When an agent is created with missing/model
    Then construction fails without dispatching a request

  Scenario: Ambiguous scripted upstream IDs are rejected
    Given scripted aliases first and second share upstream ID shared/model
    When an agent is created with shared/model
    Then construction fails without choosing either alias or dispatching a request
