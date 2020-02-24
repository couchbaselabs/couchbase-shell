(function () {
  "use strict";

  angular
    .module('mnGroupsService', [
    ])
    .factory('mnGroupsService', mnGroupsService);

  function mnGroupsService($http, $filter) {
    var mnGroupsService = {
      getGroups: getGroups,
      getGroupsState: getGroupsState,
      createGroup: createGroup,
      updateGroup: updateGroup,
      deleteGroup: deleteGroup,
      applyChanges: applyChanges,
      getGroupsByHostname: getGroupsByHostname
    };

    return mnGroupsService;

    function applyChanges(url, groups) {
      return $http({
        method: "PUT",
        url: url,
        data: JSON.stringify({"groups": groups})
      });
    }

    function deleteGroup(url) {
      return $http({
        method: "DELETE",
        url: url
      })
    }

    function updateGroup(groupName, url) {
      return $http({
        method: "PUT",
        url: url,
        data: {
          name: groupName
        }
      });
    }

    function createGroup(groupName) {
      return $http({
        method: "POST",
        url: "/pools/default/serverGroups",
        data: {
          name: groupName
        }
      });
    }

    function getGroupsState() {
      return mnGroupsService.getGroups();
    }

    function getGroupsByHostname() {
      return mnGroupsService.getGroups().then(function (resp) {
        var groups = resp.groups;
        var hostnameToGroup = {};

        _.each(groups, function (group) {
          _.each(group.nodes, function (node) {
            hostnameToGroup[node.hostname] = group;
          });
        });

        return hostnameToGroup;
      });
    }

    function getGroups() {
      return $http({
        method: 'GET',
        url: '/pools/default/serverGroups'
      }).then(function (resp) {
        var groups = $filter('orderBy')(resp.data.groups, 'name');
        resp.data.currentGroups = _.cloneDeep(groups);
        resp.data.initialGroups = _.cloneDeep(groups);
        return resp.data;
      });
    }
  }

})();
