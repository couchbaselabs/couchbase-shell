(function () {
  "use strict";

  angular
    .module("mnExceptionReporter", [])
    .config(mnExceptionReporterConfig);

  function mnExceptionReporterConfig($provide) {
    $provide.decorator('$exceptionHandler', mnExceptionReporter)
  }

  function mnExceptionReporter($delegate, $injector) {
    var errorReportsLimit = 8;
    var sentReports = 0;

    // TransitionRejection types
    // 2 "SUPERSEDED";
    // 3 "ABORTED";
    // 4 "INVALID";
    // 5 "IGNORED";
    // 6 "ERROR";
    return function (exception, cause) {
      if (
        exception.constructor.name === "Rejection" &&
       (exception.type === 2 || exception.type === 3 || exception.type === 5)
      ) {
        return; //we are not interested in these Rejection exceptions;
      }
      exception.cause = cause;
      send(exception);
      $delegate(exception, cause);
    };

    function formatErrorMessage(exception) {
      var error = ["Got unhandled javascript error:\n"];
      angular.forEach(["name", "message", "fileName", "lineNumber", "columnNumber", "stack"], function (property) {
        if (exception[property]) {
          error.push(property + ": " + exception[property] + ";\n");
        }
      });
      return error;
    }

    function send(exception) {
      if (exception.hasOwnProperty("config") &&
          exception.hasOwnProperty("headers") &&
          exception.hasOwnProperty("status") &&
          exception.hasOwnProperty("statusText")) {
        return; //we are not interested in http exception;
      }
      var error;
      if (sentReports < errorReportsLimit) {
        sentReports++;
        error = formatErrorMessage(exception);
        if (sentReports == errorReportsLimit - 1) {
          error.push("Further reports will be suppressed\n");
        }
      }
      // mozilla can report errors in some cases when user leaves current page
      // so delay report sending
      if (error) {
        _.delay(function () {
          $injector.get("$http")({
            method: "POST",
            url: "/logClientError",
            data: error.join("")
          });
        }, 500);
      }
    }
  }

})();
