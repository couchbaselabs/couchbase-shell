(function () {
  "use strict";

  angular.module('mnWizard', [
    'mnWizardService',
    'mnClusterConfigurationService',
    'mnAuthService',
    'mnSpinner',
    'mnPromiseHelper',
    'mnFocus',
    'mnAutocompleteOff',
    'mnMinlength',
    'mnEqual',
    'mnFilters',
    'mnServicesDiskPaths',
    'mnRootCertificateService'
  ]).config(mnWizardConfig);

  function mnWizardConfig($stateProvider) {
    $stateProvider
      .state('app.wizard', {
        abstract: true,
        templateUrl: 'app/mn_wizard/mn_wizard.html',
        controller: "mnWizardWelcomeController as wizardWelcomeCtl",
        resolve: {
          pools: function (mnPools) {
            return mnPools.get();
          }
        }
      })
      .state('app.wizard.welcome', {
        templateUrl: 'app/mn_wizard/welcome/mn_wizard_welcome.html'
      })
      .state('app.wizard.setupNewCluster', {
        templateUrl: 'app/mn_wizard/mn_setup_new_cluster/mn_setup_new_cluster.html',
        controller: 'mnSetupNewClusterController as setupNewClusterCtl'
      })
      .state('app.wizard.joinCluster', {
        templateUrl: 'app/mn_wizard/mn_join_cluster/mn_join_cluster.html',
        controller: 'mnClusterConfigurationController as clusterConfigurationCtl'
      })
      .state('app.wizard.termsAndConditions', {
        templateUrl: 'app/mn_wizard/mn_terms_and_conditions/mn_terms_and_conditions.html',
        controller: 'mnTermsAndConditionsController as termsCtl'
      })
      .state('app.wizard.clusterConfiguration', {
        templateUrl: 'app/mn_wizard/mn_cluster_configuration/mn_cluster_configuration.html',
        controller: 'mnClusterConfigurationController as clusterConfigurationCtl'
      })
  }
})();
