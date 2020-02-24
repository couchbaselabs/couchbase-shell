(function () {
  "use strict";

  angular
    .module('mnAdmin')
    .controller('mnAdminController', mnAdminController);

  function mnAdminController($scope, $rootScope, $state, $uibModal, mnAlertsService, poolDefault, mnSettingsNotificationsService, mnPromiseHelper, pools, mnPoller, mnEtagPoller, mnAuthService, mnTasksDetails, mnPoolDefault, mnSettingsAutoFailoverService, formatProgressMessageFilter, mnPrettyVersionFilter, mnPoorMansAlertsService, mnLostConnectionService, mnPermissions, mnPools, mnMemoryQuotaService, mnResetPasswordDialogService, whoami, mnBucketsStats, mnBucketsService, $q, mnSessionService, mnServersService, mnSettingsClusterService, mnLogsService) {
    var vm = this;
    vm.poolDefault = poolDefault;
    vm.launchpadId = pools.launchID;
    vm.implementationVersion = pools.implementationVersion;
    vm.logout = mnAuthService.logout;
    vm.resetAutoFailOverCount = resetAutoFailOverCount;
    vm.isProgressBarClosed = true;
    vm.toggleProgressBar = toggleProgressBar;
    vm.filterTasks = filterTasks;
    vm.showResetPasswordDialog = showResetPasswordDialog;
    vm.postCancelRebalanceRetry = postCancelRebalanceRetry;
    vm.showClusterInfoDialog = mnLogsService.showClusterInfoDialog;
    vm.isDeveloperPreview = pools.isDeveloperPreview;

    vm.user = whoami;

    vm.$state = $state;

    vm.enableInternalSettings = $state.params.enableInternalSettings;
    vm.runInternalSettingsDialog = runInternalSettingsDialog;
    vm.lostConnState = mnLostConnectionService.getState();

    vm.clientAlerts = mnAlertsService.clientAlerts;
    vm.alerts = mnAlertsService.alerts;
    vm.closeAlert = mnAlertsService.removeItem;
    vm.setHideNavSidebar = mnPoolDefault.setHideNavSidebar;
    vm.postStopRebalance = postStopRebalance;
    vm.closeCustomAlert = closeCustomAlert;
    vm.enableCustomAlert = enableCustomAlert;

    vm.getRebalanceReport = getRebalanceReport;

    $rootScope.rbac = mnPermissions.export;
    $rootScope.poolDefault = mnPoolDefault.export;
    $rootScope.pools = mnPools.export;
    $rootScope.buckets = mnBucketsService.export;

    activate();

    function closeCustomAlert(alertName) {
      vm.clientAlerts[alertName] = true;
    }

    function enableCustomAlert(alertName) {
      vm.clientAlerts[alertName] = false;
    }

    function postCancelRebalanceRetry(id) {
      mnSettingsClusterService.postCancelRebalanceRetry(id);
    }

    function showResetPasswordDialog() {
      vm.showUserDropdownMenu = false;
      mnResetPasswordDialogService.showDialog(whoami);
    }

    function postStopRebalance() {
      return mnPromiseHelper(vm, mnServersService.stopRebalanceWithConfirm())
        .broadcast("reloadServersPoller");
    }

    function runInternalSettingsDialog() {
      $uibModal.open({
        templateUrl: "app/mn_admin/mn_internal_settings.html",
        controller: "mnInternalSettingsController as internalSettingsCtl"
      });
    }

    function toggleProgressBar() {
      vm.isProgressBarClosed = !vm.isProgressBarClosed;
    }

    function filterTasks(runningTasks, includeRebalance) {
      return (runningTasks || []).filter(function (task) {
        return formatProgressMessageFilter(task, includeRebalance);
      });
    }

    function resetAutoFailOverCount() {
      var queries = [
        mnSettingsAutoFailoverService.resetAutoFailOverCount({group: "global"}),
        mnSettingsAutoFailoverService.resetAutoReprovisionCount({group: "global"})
      ];

      mnPromiseHelper(vm, $q.all(queries))
        .reloadState()
        .showSpinner('resetQuotaLoading')
        .catchGlobalErrors('Unable to reset Auto-failover quota!')
        .showGlobalSuccess("Auto-failover quota reset successfully!");
    }

    function getRebalanceReport() {
      mnTasksDetails.getRebalanceReport().then(function(report) {
        var file = new Blob([JSON.stringify(report,null,2)],{type: "application/json", name: "rebalanceReport.json"});
        saveAs(file,"rebalanceReport.json");
      });
    }

    function activate() {

      new mnPoller($scope, function () {
        return mnBucketsService.findMoxiBucket();
      })
        .subscribe("moxiBucket", vm)
        .reloadOnScopeEvent(["reloadBucketStats"])
        .cycle();

      if (mnPermissions.export.cluster.settings.read) {
        new mnPoller($scope, function () {
          return mnSettingsAutoFailoverService.getAutoFailoverSettings();
        })
          .setInterval(10000)
          .subscribe("autoFailoverSettings", vm)
          .reloadOnScopeEvent(["reloadServersPoller", "rebalanceFinished"])
          .cycle();
      }

      mnSessionService.init($scope);

      if (mnPermissions.export.cluster.settings.read) {
        mnPromiseHelper(vm, mnSettingsNotificationsService.maybeCheckUpdates({group: "global"}))
          .applyToScope("updates")
          .onSuccess(function (updates) {
            if (updates.sendStats) {
              mnPromiseHelper(vm, mnSettingsNotificationsService.buildPhoneHomeThingy({group: "global"}))
                .applyToScope("launchpadSource")
            }
          });
      }

      var etagPoller = new mnEtagPoller($scope, function (previous) {
        return mnPoolDefault.get({
          etag: previous ? previous.etag : "",
          waitChange: $state.current.name === "app.admin.overview.statistics" ? 3000 : 10000
        }, {group: "global"});
      }, true).subscribe(function (resp, previous) {
        if (!_.isEqual(resp, previous)) {
          $rootScope.$broadcast("mnPoolDefaultChanged");
        }

        if (Number(localStorage.getItem("uiSessionTimeout")) !== (resp.uiSessionTimeout * 1000)) {
          $rootScope.$broadcast("newSessionTimeout", resp.uiSessionTimeout);
        }

        vm.tabName = resp.clusterName;

        if (previous && !_.isEqual(resp.nodes, previous.nodes)) {
          $rootScope.$broadcast("nodesChanged", [resp.nodes, previous.nodes]);
        }

        if (previous && previous.buckets.uri !== resp.buckets.uri) {
          $rootScope.$broadcast("reloadBucketStats");
        }

        if (previous && previous.serverGroupsUri !== resp.serverGroupsUri) {
          $rootScope.$broadcast("serverGroupsUriChanged");
        }

        if (previous && previous.indexStatusURI !== resp.indexStatusURI) {
          $rootScope.$broadcast("indexStatusURIChanged");
        }

        if (!_.isEqual(resp.alerts, (previous || {}).alerts)) {
          mnPoorMansAlertsService.maybeShowAlerts(resp);
        }

        var version = mnPrettyVersionFilter(pools.implementationVersion);
        $rootScope.mnTitle = vm.tabName + (version ? (' - ' + version) : '');

        if (previous && previous.tasks.uri != resp.tasks.uri) {
          $rootScope.$broadcast("reloadTasksPoller");
        }

        if (previous && previous.checkPermissionsURI != resp.checkPermissionsURI) {
          $rootScope.$broadcast("reloadPermissions");
        }
      })
          .cycle();

      if (mnPermissions.export.cluster.tasks.read) {
        if (pools.isEnterprise && poolDefault.compat.atLeast65) {
          var retryRebalancePoller = new mnPoller($scope, function () {
            return mnSettingsClusterService.getPendingRetryRebalance({group: "global"});
          })
              .setInterval(function (resp) {
                return resp.data.retry_after_secs ? 1000 : 3000;
              })
              .subscribe(function (resp) {
                vm.retryRebalance = resp.data;
              }).cycle();
        }

        var tasksPoller = new mnPoller($scope, function (prevTask) {
          return mnTasksDetails.getFresh({group: "global"})
            .then(function (tasks) {
              if (poolDefault.compat.atLeast65) {
                if (tasks.tasksRebalance.status == "notRunning") {
                  if (!tasks.tasksRebalance.masterRequestTimedOut &&
                      prevTask && (tasks.tasksRebalance.lastReportURI !=
                                   prevTask.tasksRebalance.lastReportURI)) {
                    mnTasksDetails.clearRebalanceReportCache(prevTask.tasksRebalance.lastReportURI);
                  }
                  return mnTasksDetails.getRebalanceReport(tasks.tasksRebalance.lastReportURI)
                    .then(function (rv) {
                      if (rv.data.stageInfo) {
                        tasks.tasksRebalance.stageInfo = rv.data.stageInfo;
                        tasks.tasksRebalance.completionMessage = rv.data.completionMessage;
                      }
                      return tasks;
                    });
                }
                return tasks;
              }
              return tasks;
            });
        })
            .setInterval(function (result) {
              return (_.chain(result.tasks).pluck('recommendedRefreshPeriod').compact().min().value() * 1000) >> 0 || 10000;
            })
            .subscribe(function (tasks, prevTask) {
              vm.showTasksSpinner = false;
              if (!_.isEqual(tasks, prevTask)) {
                $rootScope.$broadcast("mnTasksDetailsChanged");
              }

              var isRebalanceFinished =
                  tasks.tasksRebalance && tasks.tasksRebalance.status !== 'running' &&
                  prevTask && prevTask.tasksRebalance && prevTask.tasksRebalance.status === "running";
              if (isRebalanceFinished) {
                $rootScope.$broadcast("rebalanceFinished");
              }

              if (!vm.isProgressBarClosed &&
                  !filterTasks(tasks.running).length &&
                  !tasks.tasksRebalance.stageInfo &&
                  prevTask && filterTasks(prevTask.running).length) {
                vm.isProgressBarClosed = true;
              }

              var stageInfo = {
                services: {},
                startTime: null,
                completedTime: {
                  status: true
                }
              };
              var serverStageInfo = tasks.tasksRebalance.stageInfo ||
                  (tasks.tasksRebalance.previousRebalance &&
                   tasks.tasksRebalance.previousRebalance.stageInfo);

              if (serverStageInfo) {
                var services = Object
                    .keys(serverStageInfo)
                    .sort(function (a, b) {
                      if (!serverStageInfo[a].timeTaken) {
                        return 1;
                      }
                      if (!serverStageInfo[b].startTime) {
                        return -1;
                      }
                      if (new Date(serverStageInfo[a].startTime) >
                          new Date(serverStageInfo[b].startTime)) {
                        return 1;
                      } else {
                        return -1;
                      }
                    });

                stageInfo.services = services
                  .map(function(key) {
                    var value = serverStageInfo[key];
                    value.name = key;
                    var details = Object
                        .keys(value.details || {})
                        // .sort(function (a, b) {
                        //   return new Date(value.details[a].startTime) -
                        //     new Date(value.details[b].startTime);
                        // });

                    value.details = details.map(function (bucketName) {
                      value.details[bucketName].name = bucketName;
                      return value.details[bucketName];
                    });

                    if (value.startTime) {
                      if (!stageInfo.startTime ||
                          stageInfo.startTime > new Date(value.startTime)) {
                        stageInfo.startTime = new Date(value.startTime);
                      }
                    }
                    if (value.completedTime) {
                      value.completedTime = new Date(value.completedTime);
                      if (!stageInfo.completedTime.time ||
                          (stageInfo.completedTime.time < value.completedTime)) {
                        stageInfo.completedTime.time = new Date(value.completedTime);
                      }
                    } else {
                      stageInfo.completedTime.status = false;
                    }
                    return value;
                  });

                tasks.tasksRebalance.stageInfo = stageInfo;
              }

              if (tasks.inRebalance) {
                if (!prevTask) {
                  vm.isProgressBarClosed = false;
                } else {
                  if (!prevTask.tasksRebalance ||
                      prevTask.tasksRebalance.status !== "running") {
                    vm.isProgressBarClosed = false;
                  }
                }
              }

              if (tasks.tasksRebalance.errorMessage && mnAlertsService.isNewAlert({id: tasks.tasksRebalance.statusId})) {
                mnAlertsService.setAlert("error", tasks.tasksRebalance.errorMessage, null, tasks.tasksRebalance.statusId);
              }
              vm.tasks = tasks;
            }, vm)
            .cycle();
      }

      $scope.$on("reloadPermissions", function () {
        mnPermissions.getFresh();
      });

      $scope.$on("newSessionTimeout", function (e, uiSessionTimeout) {
        mnSessionService.setTimeout(uiSessionTimeout);
        mnSessionService.resetTimeoutAndSyncAcrossTabs();
      });

      $scope.$on("reloadTasksPoller", function (event, params) {
        if (!params || !params.doNotShowSpinner) {
          vm.showTasksSpinner = true;
        }
        if (tasksPoller) {
          tasksPoller.reload(true);
        }
      });

      $scope.$on("reloadPoolDefaultPoller", function () {
        mnPoolDefault.clearCache();
        etagPoller.reload();
      });

      $scope.$on("reloadBucketStats", function () {
        mnBucketsService.clearCache();
        mnBucketsService.getBucketsByType();
      });
      $rootScope.$broadcast("reloadBucketStats");

      $scope.$on("maybeShowMemoryQuotaDialog", function (event, services) {
        return mnPoolDefault.get().then(function (poolsDefault) {
          var servicesToCheck = ["index", "fts"];
          if (poolsDefault.isEnterprise) {
            servicesToCheck = servicesToCheck.concat(["cbas", "eventing"]);
          }
          var firstTimeAddedServices =
              mnMemoryQuotaService
              .getFirstTimeAddedServices(servicesToCheck, services, poolsDefault.nodes);
          if (firstTimeAddedServices.count) {
            $uibModal.open({
              windowTopClass: "without-titlebar-close",
              backdrop: 'static',
              templateUrl: 'app/mn_admin/memory_quota_dialog.html',
              controller: 'mnServersMemoryQuotaDialogController as serversMemoryQuotaDialogCtl',
              resolve: {
                memoryQuotaConfig: function (mnMemoryQuotaService) {
                  return mnMemoryQuotaService.memoryQuotaConfig(services, true, false);
                },
                indexSettings: function (mnSettingsClusterService) {
                  return mnSettingsClusterService.getIndexSettings();
                },
                firstTimeAddedServices: function() {
                  return firstTimeAddedServices;
                }
              }
            });
          }
        });
      });
    }
  }
})();
