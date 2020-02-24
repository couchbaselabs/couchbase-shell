/**
 * Reactive extensions for UI-Router
 * @version v0.5.0
 * @link https://github.com/ui-router/rx#readme
 * @license MIT License, http://www.opensource.org/licenses/MIT
 */
(function (global, factory) {
    typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports, require('rxjs'), require('rxjs/operators')) :
    typeof define === 'function' && define.amd ? define(['exports', 'rxjs', 'rxjs/operators'], factory) :
    (factory((global['@uirouter/rx'] = {}),global.rxjs,global.rxjs.operators));
}(this, (function (exports,rxjs,operators) { 'use strict';

    /** @module rx */
    /** Augments UIRouterGlobals with observables for transition starts, successful transitions, and state parameters */
    var UIRouterRx = /** @class */ (function () {
        function UIRouterRx(router) {
            this.name = '@uirouter/rx';
            this.deregisterFns = [];
            var start$ = new rxjs.ReplaySubject(1);
            var success$ = start$.pipe(operators.mergeMap(function (t) { return t.promise.then(function () { return t; }, function () { return null; }); }), operators.filter(function (t) { return !!t; }));
            var params$ = success$.pipe(operators.map(function (transition) { return transition.params(); }));
            var states$ = new rxjs.ReplaySubject(1);
            function onStatesChangedEvent(event, states) {
                var changeEvent = {
                    currentStates: router.stateRegistry.get(),
                    registered: [],
                    deregistered: [],
                };
                if (event)
                    changeEvent[event] = states;
                states$.next(changeEvent);
            }
            this.deregisterFns.push(router.transitionService.onStart({}, function (transition) { return start$.next(transition); }));
            this.deregisterFns.push(router.stateRegistry.onStatesChanged(onStatesChangedEvent));
            onStatesChangedEvent(null, null);
            Object.assign(router.globals, { start$: start$, success$: success$, params$: params$, states$: states$ });
        }
        UIRouterRx.prototype.dispose = function () {
            this.deregisterFns.forEach(function (deregisterFn) { return deregisterFn(); });
            this.deregisterFns = [];
        };
        return UIRouterRx;
    }());
    var UIRouterRxPlugin = UIRouterRx;

    exports.UIRouterRx = UIRouterRx;
    exports.UIRouterRxPlugin = UIRouterRxPlugin;

    Object.defineProperty(exports, '__esModule', { value: true });

})));
//# sourceMappingURL=ui-router-rx.js.map
