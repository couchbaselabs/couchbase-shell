(function () {
  "use strict";

  angular
    .module('mnLaunchpad', [])
    .directive('mnLaunchpad', mnLaunchpadDirective);

  function mnLaunchpadDirective($timeout) {
    var mnLaunchpad = {
      scope: {
        launchpadSource: "=",
        launchpadId: "="
      },
      link: link
    }

    return mnLaunchpad;

    function link($scope, $element, $attrs) {
      $scope.$watch('launchpadSource', function (launchpadSource) {
        if (!launchpadSource) {
          return;
        }
        var iframe = document.createElement("iframe");
        iframe.style.display = 'none'
        $element.append(iframe);
        var idoc = iframe.contentWindow.document;
        idoc.body.innerHTML = "<form id=\"launchpad\" method=\"POST\"><textarea id=\"sputnik\" name=\"stats\"></textarea></form>";
        var form = idoc.getElementById("launchpad");
        var textarea = idoc.getElementById("sputnik");
        form['action'] = "https://ph.couchbase.net/v2?launchID=" + $scope.launchpadId;
        textarea.innerText = JSON.stringify(launchpadSource);
        form.submit();
        $scope.launchpadSource = undefined;

        $timeout(function () {
          $element.empty();
        }, 30000);
      });
    }
  }
})();
