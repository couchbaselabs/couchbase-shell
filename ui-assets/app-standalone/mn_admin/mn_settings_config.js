(function () {
  "use strict";

  angular
    .module('mnSettings', [
      'mnSettingsNotifications',
      'mnSettingsSampleBuckets',
      'mnSettingsCluster',
      'mnSettingsAutoFailover',
      'mnSettingsAutoCompaction',
      'mnAudit',
      'mnSettingsCluster',
      'mnSettingsAlerts',
      'mnSettingsNotificationsService',
      'ui.router',
      'mnPluggableUiRegistry',
      'mnElementCrane',
      'mnSession'
    ])
    .config(mnSettingsConfig);

  function mnSettingsConfig($stateProvider) {

    $stateProvider
      .state('app.admin.settings', {
        url: '/settings',
        abstract: true,
        views: {
          "main@app.admin": {
            templateUrl: 'app/mn_admin/mn_settings.html',
            controller: 'mnSettingsController as settingsCtl'
          }
        },
        data: {
          title: "Settings"
        }
      })
      .state('app.admin.settings.cluster', {
        url: '/cluster',
        views: {
          "": {
            controller: 'mnSettingsClusterController as settingsClusterCtl',
            templateUrl: 'app/mn_admin/mn_settings_cluster.html'
          },
          "autofailover@app.admin.settings.cluster": {
            controller: 'mnSettingsAutoFailoverController as settingsAutoFailoverCtl',
            templateUrl: 'app/mn_admin/mn_settings_auto_failover.html'
          },
          "notifications@app.admin.settings.cluster": {
            controller: 'mnSettingsNotificationsController as settingsNotificationsCtl',
            templateUrl: 'app/mn_admin/mn_settings_notifications.html'
          }
        }
      })
      .state('app.admin.settings.alerts', {
        url: '/alerts',
        controller: 'mnSettingsAlertsController as settingsAlertsCtl',
        templateUrl: 'app/mn_admin/mn_settings_alerts.html',
        data: {
          permissions: 'cluster.settings.read'
        }
      })
      .state('app.admin.settings.autoCompaction', {
        url: '/autoCompaction',
        controller: 'mnSettingsAutoCompactionController as settingsAutoCompactionCtl',
        templateUrl: 'app/mn_admin/mn_settings_auto_compaction.html',
        data: {
          permissions: 'cluster.settings.read'
        }
      })
      .state('app.admin.settings.sampleBuckets', {
        url: '/sampleBuckets',
        controller: 'mnSettingsSampleBucketsController as settingsSampleBucketsCtl',
        templateUrl: 'app/mn_admin/mn_settings_sample_buckets.html',
        data: {
          permissions: 'cluster.samples.read'
        }
      });
  }
})();
