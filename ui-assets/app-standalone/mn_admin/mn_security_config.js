(function () {
  "use strict";

  angular.module('mnSecurity', [
    'mnUserRoles',
    'mnPluggableUiRegistry',
    'mnRootCertificate',
    'mnElementCrane',
    'mnClientCertificate',
    'mnRedaction',
    'mnRolesGroups'
  ]).config(mnIndexesConfig);

  function mnIndexesConfig($stateProvider) {
    $stateProvider
      .state('app.admin.security', {
        abstract: true,
        url: "/security",
        views: {
          "main@app.admin": {
            controller: "mnSecurityController as securityCtl",
            templateUrl: "app/mn_admin/mn_security.html"
          }
        },
        data: {
          permissions: "cluster.admin.security.read",
          title: "Security"
        }
      })
      .state('app.admin.security.roles', {
        abstract: true,
        templateUrl: "app/mn_admin/mn_roles.html",
        controller: "mnRolesController as rolesCtl"
      })
      .state('app.admin.security.roles.user', {
        url: "/userRoles?openedUsers&startFrom&startFromDomain&sortBy&order&substr&{pageSize:int}",
        params: {
          openedUsers: {
            array: true,
            dynamic: true
          },
          substr: {
            dynamic: true,
            value: ""
          },
          pageSize: {
            value: 20
          },
          startFrom: {
            value: null
          },
          startFromDomain: {
            value: null
          },
          sortBy: {
            value: "id",
            dynamic: true
          },
          order: {
            value: "asc",
            dynamic: true
          }
        },
        controller: "mnUserRolesController as userRolesCtl",
        templateUrl: "app/mn_admin/mn_user_roles.html"
      })
      .state('app.admin.security.roles.groups', {
        // url: "/userRoles?openedUsers&startFrom&startFromDomain&{pageSize:int}",
        url: "/rolesGroups?startFrom&sortBy&order&substr&{pageSize:int}",
        params: {
          openedRolesGroups: {
            array: true,
            dynamic: true
          },
          substr: {
            dynamic: true,
            value: ""
          },
          pageSize: {
            value: 20
          },
          startFrom: {
            value: null
          },
          sortBy: {
            value: "id",
            dynamic: true
          },
          order: {
            value: "asc",
            dynamic: true
          }
        },
        controller: "mnRolesGroupsController as rolesGroupsCtl",
        templateUrl: "app/mn_admin/mn_roles_groups.html",
        data: {
          enterprise: true
        }
      })
      .state('app.admin.security.session', {
        url: '/session',
        controller: 'mnSessionController as sessionCtl',
        templateUrl: 'app/mn_admin/mn_session.html',
        data: {
          permissions: "cluster.admin.security.read"
        }
      })
      .state('app.admin.security.rootCertificate', {
        url: '/rootCertificate',
        controller: 'mnRootCertificateController as rootCertificateCtl',
        templateUrl: 'app/mn_admin/mn_root_certificate.html',
        data: {
          enterprise: true
        }
      })
      .state('app.admin.security.clientCert', {
        url: '/clientCert',
        controller: 'mnClientCertController as clientCertCtl',
        templateUrl: 'app/mn_admin/mn_client_certificate.html',
        data: {
          enterprise: true
        }
      })
      .state('app.admin.security.audit', {
        url: '/audit',
        controller: 'mnAuditController as auditCtl',
        templateUrl: 'app/mn_admin/mn_audit.html',
        data: {
          enterprise: true
        }
      })
      .state('app.admin.security.redaction', {
        url: '/redaction',
        controller: 'mnRedactionController as redactionCtl',
        templateUrl: 'app/mn_admin/mn_redaction.html',
        data: {
          compat: "atLeast55",
          enterprise: true
        }
      });
  }
})();
