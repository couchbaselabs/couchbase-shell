(function () {
  "use strict";
  angular
    .module('mnWizard')
    .controller('mnClusterConfigurationController', mnClusterConfigurationController);

  function mnClusterConfigurationController($scope, $rootScope, $state, $q, mnClusterConfigurationService, mnSettingsClusterService, mnAuthService, pools, mnHelper, mnServersService, mnPools, mnAlertsService, mnPromiseHelper, mnWizardService, mnStatisticsNewService, mnRootCertificateService) {
    var vm = this;

    vm.joinClusterConfig = mnClusterConfigurationService.getJoinClusterConfig();
    vm.defaultJoinClusterSerivesConfig = _.clone(vm.joinClusterConfig.services, true);
    vm.isEnterprise = pools.isEnterprise;
    vm.hostConfig = {
      afamily: "ipv4",
      nodeEncryption: 'off'
    };

    vm.onSubmit = onSubmit;
    vm.onIPvChange = onIPvChange;
    vm.sendStats = true;

    activate();

    function postSetupNetConfig() {
      return mnClusterConfigurationService.postSetupNetConfig(vm.hostConfig);
    }
    function postEnableExternalListener() {
      return mnClusterConfigurationService.postEnableExternalListener(vm.hostConfig);
    }
    function onIPvChange() {
      if (vm.hostConfig.afamily == "ipv6" && vm.config.hostname == "127.0.0.1") {
        vm.config.hostname = "::1";
      }
      if (vm.hostConfig.afamily == "ipv4" && vm.config.hostname == "::1") {
        vm.config.hostname = "127.0.0.1";
      }
    }
    function postHostConfig() {
      var promise = postEnableExternalListener().then(postSetupNetConfig);
      return addErrorHandler(promise, "postHostConfig");
    }

    function activate() {
      if (!mnWizardService.getState().isNewCluster && vm.isEnterprise) {
        mnPromiseHelper(vm, mnRootCertificateService.getDefaultCertificate())
          .applyToScope("certificate");
      }

      mnPromiseHelper(vm, mnClusterConfigurationService.getConfig())
        .applyToScope("config")
        .onSuccess(function (config) {
          vm.defaultConfig = _.clone(config);
          vm.hostConfig = {
            afamily: vm.defaultConfig.addressFamily == 'inet6' ? "ipv6" : "ipv4",
            nodeEncryption: vm.defaultConfig.nodeEncryption ? 'on' : 'off'
          };
          onIPvChange();
        });

      mnPromiseHelper(vm, mnClusterConfigurationService.getQuerySettings())
        .applyToScope("querySettings");

      $scope.$watch('clusterConfigurationCtl.config.startNewClusterConfig', _.debounce(onMemoryQuotaChanged, 500), true);
    }

    function onMemoryQuotaChanged(memoryQuotaConfig) {
      if (!memoryQuotaConfig) {
        return;
      }
      var promise = mnSettingsClusterService.postPoolsDefault(memoryQuotaConfig, true);
      mnPromiseHelper(vm, promise)
        .catchErrorsFromSuccess("postMemoryErrors");
    }

    function goNext() {
      var newClusterState = mnWizardService.getNewClusterState();
      return mnClusterConfigurationService.postAuth(newClusterState.user).then(function () {
        return mnAuthService.login(newClusterState.user).then(function () {
          var config = mnClusterConfigurationService.getNewClusterConfig();
          if (config.services.model.index) {
            mnSettingsClusterService.postIndexSettings(config.indexSettings);
          }
        }).then(function () {
          return $state.go('app.admin.overview.statistics');
        });
      });
    }
    function addErrorHandler(query, name) {
      return mnPromiseHelper(vm, query)
        .catchErrors(name + 'Errors')
        .getPromise();
    }
    function postMemoryQuota() {
      var data = _.clone(vm.config.startNewClusterConfig);
      var newClusterState = mnWizardService.getNewClusterState();
      !vm.config.startNewClusterConfig.services.model.index && (delete data.indexMemoryQuota);
      !vm.config.startNewClusterConfig.services.model.fts && (delete data.ftsMemoryQuota);

      if (pools.isEnterprise) {
        !vm.config.startNewClusterConfig.services.model.cbas && (delete data.cbasMemoryQuota);
        !vm.config.startNewClusterConfig.services.model.eventing && (delete data.eventingMemoryQuota);
      }
      return addErrorHandler(mnSettingsClusterService.postPoolsDefault(data, false, newClusterState.clusterName), "postMemory");
    }
    function validateIndexSettings() {
      return mnPromiseHelper(vm, mnSettingsClusterService.postIndexSettings(vm.config.startNewClusterConfig.indexSettings))
        .catchErrors('postIndexSettingsErrors')
        .getPromise();
    }
    function postServices() {
      return addErrorHandler(mnServersService.setupServices({
        services: mnHelper.checkboxesToList(vm.config.startNewClusterConfig.services.model).join(',')
      }), "setupServices");
    }
    function postDiskStorage() {
      var data = {
        path: vm.config.dbPath,
        index_path: vm.config.indexPath,
        eventing_path: vm.config.eventingPath,
        java_home: vm.config.java_home
      };
      if (pools.isEnterprise) {
        data.cbas_path = vm.config.cbasDirs;
      }
      return addErrorHandler(
        mnClusterConfigurationService.postDiskStorage(data),
        "postDiskStorage");
    }
    function postJoinCluster() {
      var data = _.clone(vm.joinClusterConfig.clusterMember);
      data.services = mnHelper.checkboxesToList(vm.joinClusterConfig.services.model).join(',');
      data.newNodeHostname = vm.config.hostname;
      return addErrorHandler(mnClusterConfigurationService.postJoinCluster(data), "postJoinCluster");
    }
    function postStats() {
      var promise = mnClusterConfigurationService.postStats(vm.sendStats);

      return mnPromiseHelper(vm, promise)
        .catchGlobalErrors()
        .getPromise();
    }
    function doStartNewCluster() {
      var newClusterParams = vm.config.startNewClusterConfig;
      var hadServicesString = vm.config.selfConfig.services.sort().join("");
      var hasServicesString = mnHelper.checkboxesToList(newClusterParams.services.model).sort().join("");
      if (hadServicesString === hasServicesString) {
        return postMemoryQuota().then(postStats).then(goNext);
      } else {
        var hadIndexService = hadServicesString.indexOf("index") > -1;
        var hasIndexService = hasServicesString.indexOf("index") > -1;
        if (hadIndexService && !hasIndexService) {
          return postServices().then(function () {
            return postMemoryQuota().then(postStats).then(goNext);
          });
        } else {
          return postMemoryQuota().then(function () {
            return postServices().then(postStats).then(goNext);
          });
        }
      }
    }
    function onSubmit(e) {
      if (vm.viewLoading) {
        return;
      }
      delete vm.setupServicesErrors;
      delete vm.postMemoryErrors;
      delete vm.postDiskStorageErrors;
      delete vm.postJoinClusterErrors;
      delete vm.postHostnameErrors;
      delete vm.postIndexSettingsErrors;

      var promise =
          postDiskStorage().then(function () {
            if (mnWizardService.getState().isNewCluster && vm.isEnterprise) {
                return postHostConfig();
            }
          }).then(function () {
            if (mnWizardService.getState().isNewCluster) {
              return addErrorHandler(mnClusterConfigurationService
                                     .postHostname(vm.config.hostname), "postHostname");
            }
          }).then(function () {
            if (mnWizardService.getState().isNewCluster) {
              if (vm.config.startNewClusterConfig.services.model.index) {
                return validateIndexSettings().then(function () {
                  if (vm.postIndexSettingsErrors) {
                    return $q.reject();
                  }
                  return doStartNewCluster();
                });
              } else {
                return doStartNewCluster();
              }
            } else {
              return postJoinCluster().then(function () {
                return mnAuthService.login(vm.joinClusterConfig.clusterMember).then(function () {
                  return $state.go('app.admin.overview.statistics').then(function () {
                    $rootScope.$broadcast("maybeShowMemoryQuotaDialog", vm.joinClusterConfig.services.model);
                    mnAlertsService.formatAndSetAlerts('This server has been associated with the cluster and will join on the next rebalance operation.', 'success', 60000);
                  });
                });
              });
            }
          });

      mnPromiseHelper(vm, promise)
        .showGlobalSpinner();
    };
  }
})();
