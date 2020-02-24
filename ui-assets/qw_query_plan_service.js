(function() {

  //
  // the qwQueryPlanService contains utility functions for processing N1QL query
  // plans (a JSON tree-like structure) into other, more useful forms.
  //

  angular.module('qwQuery').factory('qwQueryPlanService', getQwQueryPlanService);

  getQwQueryPlanService.$inject = [];

  function getQwQueryPlanService() {

    const analyticsFieldPattern = /getField\((.+?)\)/igm;
    var qwQueryPlanService = {};

    //
    qwQueryPlanService.convertN1QLPlanToPlanNodes = convertN1QLPlanToPlanNodes;
    qwQueryPlanService.convertAnalyticsPlanToPlanNodes = convertAnalyticsPlanToPlanNodes;
    qwQueryPlanService.analyzePlan = analyzePlan;
    qwQueryPlanService.analyzeAnalyticsPlan = analyzeAnalyticsPlan;
    qwQueryPlanService.convertTimeToNormalizedString = convertTimeToNormalizedString;
    qwQueryPlanService.convertTimeFloatToFormattedString = convertTimeFloatToFormattedString;
    qwQueryPlanService.convertTimeStringToFloat = convertTimeStringToFloat;

    //
    // convertAnalyticsPlanToPlanNodes
    //
    //

    function convertAnalyticsPlanToPlanNodes(plan, predecessor, lists) {

      // sanity check
      if (!plan || _.isString(plan))
        return(null);

      // get the "#operator" field
      var operatorName = plan['#operator'];

      // we had better have an operator name at this point
      if (!operatorName) {
        console.log("Error, no operator found for item, plan: " + JSON.stringify(plan));
        console.log(JSON.stringify(plan));
        return(null);
      }

      // ignore 'empty-tuple-source' operator
      if (operatorName === "empty-tuple-source")
        return(null);

      // bypass 'exchange' operators
      if (operatorName === "exchange" && plan['physical-operator'] === "ONE_TO_ONE_EXCHANGE" &&
          _.isArray(plan['inputs']) && plan['inputs'].length == 1)
        return(convertAnalyticsPlanToPlanNodes(plan['inputs'][0], predecessor, lists));

      // analytics operator with inputs
      if (plan["inputs"]) {
        var inputs = plan["inputs"];
        var opInputs = [];
        for (var i = 0; i < inputs.length; i++) {
          var input = convertAnalyticsPlanToPlanNodes(inputs[i], predecessor, lists);
          if (input)
            opInputs.push(input);
        }
        var op = new PlanNode(opInputs, plan, null, null);
        return (op);
      }

      // for all other operators, create a plan node
      else {
        return(new PlanNode(predecessor,plan,null,lists.total_time));
      }
    }


    //
    //
    //

    function analyzeAnalyticsPlan(plan,lists) {
      if (!lists)
        lists = {buckets : {}, fields : {}, indexes: {}, aliases: [], datasets: {}, total_time: 0.0};

      if (!plan || _.isString(plan))
        return(null);

      // iterate over fields, look for "#operator" field
      var operatorName = plan['#operator'];
      //console.log("Analyzing plan node: " + operatorName);

      // at this point we should have an operation name and a field array
      //console.log("  after analyze, got op name: " + operatorName);
      // we had better have an operator name at this point

      if (!operatorName) {
        console.log("Error, no operator found for item: " + JSON.stringify(plan));
        return(lists);
      }

      else if (plan["data-source"]) {
        lists.datasets[plan["data-source"]] = true;
      }

      else if (plan["expressions"]) {
        var exp = plan["expressions"];
        if (_.isArray(exp)) {
          for (let i = 0; i < exp.length; i++) {
            var exprStr = exp[i];
            processExpressionsField(exprStr, lists.datasets, lists.indexes, lists.fields);
          }
        } else {
          processExpressionsField(exp, lists.datasets, lists.indexes, lists.fields);
        }
      }

      // analytics operator with inputs
      if (plan["inputs"]) {
        var inputs = plan['inputs'];
        for (var i = 0; i < inputs.length; i++) {
          analyzeAnalyticsPlan(inputs[i], lists);
        }
      }

      return (lists);
    }


    //
    //
    //

    function processExpressionsField(exprStr, datasets, indexes, fields) {
      if (_.isString(exprStr)) {
        if (exprStr.startsWith("index-search(")) {
          extractSourceFromExpression(exprStr, datasets, indexes);
        } else if (exprStr.includes(".getField")) {
          extractFieldsFromExpression(exprStr, fields);
        }
      }
    }

    function extractSourceFromExpression(expression, datasets, indexes) {
      let idxBegin = expression.indexOf("(") + 1;
      let params = expression.substring(idxBegin).split(",");
      let indexName = params[0].trim();
      let dataverseName = params[2].trim();
      let datasetName = params[3].trim();
      let fullyQualifiedName = dataverseName + "." + indexName;
      if(indexName === datasetName) {
        datasets[fullyQualifiedName] = true;
      } else {
        indexes[fullyQualifiedName] = true;
      }
    }

    function extractFieldsFromExpression(expression, fields) {
      let matches = (expression.match(analyticsFieldPattern) || []).map(e => e.replace(analyticsFieldPattern, '$1'));
      for (let i = 0; i < matches.length; i++) {
        fields[matches[i].trim()] = true;
      }
    }

    //
    // convertN1QLPlanToPlanNodes
    //
    // We need to take the query plan, which is a somewhat arbitrary JSON
    // structure and turn it into more of a data-flow tree of PlanNodes, where
    // the root of the tree is the final output of the query, and the root's
    // children are those operators that feed data in to the result, all the way
    // back to the leaves which are the original data scans.
    //
    // usually, elements in JSON all have #operator fields, but in the case
    // of prepared queries, the tree starts as a field called "operator"
    //
    // Some nodes have children that must be traversed:
    //   Sequence has '~children'
    //   Parallel has '~child'
    //   UnionAll has '~children'
    //   UnionScan/IntersectScan have 'scans'
    //   ExceptAll/IntersetAll have 'first' and 'second'
    //   DistinctScan has 'scan'
    //   Authorize has '~child'
    //   Merge has 'as', 'key', 'keyspace', 'delete' and 'update'
    //
    //  Update has 'set_terms' (array of {"path":"...","value":"..."}),
    //             'unset_terms' (array of {"path":"..."})
    //  Let?

    function convertN1QLPlanToPlanNodes(plan, predecessor, lists) {

      // sanity check
      if (!plan || _.isString(plan))
        return(null);

      // special case: prepared queries

      if (plan.operator)
        return(convertN1QLPlanToPlanNodes(plan.operator,null,lists));

      // special case #2: plan with query timings is wrapped in an outer object
      if (plan.plan && !plan['#operator'])
        return(convertN1QLPlanToPlanNodes(plan.plan,null,lists));

      //console.log("Inside analyzePlan");

      // iterate over fields, look for "#operator" field
      var operatorName = plan['#operator'];

      // at this point we should have an operation name and a field array

      //console.log("  after analyze, got op name: " + operatorName);

      // we had better have an operator name at this point

      if (!operatorName) {
        console.log("Error, no operator found for item, plan: " + JSON.stringify(plan));
        console.log(JSON.stringify(plan));
        return(null);
      }

      // if we have a sequence, we analyze the children and append them to the predecessor
      if (operatorName === "Sequence" && plan['~children']) {
        for (var i = 0; i < plan['~children'].length; i++)
          predecessor = convertN1QLPlanToPlanNodes(plan['~children'][i],predecessor,lists);

        return(predecessor);
      }

      // parallel groups are like sequences. We used to wrap them in a separate Node, but
      // that is not really needed, we will just mark the beginning and end.

      else if (operatorName === "Parallel" && plan['~child']) {
        //console.log("Got Parallel block, predecessor: " + JSON.stringify(predecessor));
        var subsequence = convertN1QLPlanToPlanNodes(plan['~child'],predecessor,lists);
        var subseq_end = null;

        // mark the elements of a parallel subsequence for later annotation
        for (var subNode = subsequence; subNode != null; subNode = subNode.predecessor) {
          if (subNode == subsequence)
            subNode.parallelBegin = true;
          if (subNode.predecessor == predecessor) {
            subNode.parallelEnd = true;
            subseq_end = subNode;
          }
          subNode.parallel = true;
        }
        //subseqence.predecessor = predecessor;
        return(subsequence);
        //return(new PlanNode(predecessor,plan,subsequence,lists.total_time));
      }

      // Prepare operators have their plan inside prepared.operator
      else if (operatorName === "Prepare" && plan.prepared && plan.prepared.operator) {
        return(convertN1QLPlanToPlanNodes(plan.prepared.operator,null,lists));
      }

      // ExceptAll and InterceptAll have 'first' and 'second' subqueries
      else if (operatorName === "ExceptAll" || operatorName === "IntersectAll") {
        var children = [];

        if (plan['first'])
          children.push(convertN1QLPlanToPlanNodes(plan['first'],null,lists));

        if (plan['second'])
          children.push(convertN1QLPlanToPlanNodes(plan['second'],null,lists));

        if (children.length > 0)
          return(new PlanNode(children,plan,null,lists.total_time));
        else
          return(null);
      }

      // Merge may have various children: 'insert', 'delete' and/or 'update'
      else if (operatorName === "Merge") {
        var children = [];

        if (predecessor)
          children.push(predecessor);

        if (plan['insert'])
          children.push(convertN1QLPlanToPlanNodes(plan['insert'],null,lists));

        if (plan['delete'])
          children.push(convertN1QLPlanToPlanNodes(plan['delete'],null,lists));

        if (plan['update'])
          children.push(convertN1QLPlanToPlanNodes(plan['update'],null,lists));

        if (children.length > 0)
          return(new PlanNode(children,plan,null,lists.total_time));
        else
          return(null);
      }

      // Authorize operators have a single child called '~child', the child comes *after*
      // the authorize op
      else if (operatorName === "Authorize" && plan['~child']) {
        var authorizeNode = new PlanNode(predecessor,plan,null,lists.total_time);
        var authorizeChildren = convertN1QLPlanToPlanNodes(plan['~child'],authorizeNode,lists);
        return(authorizeChildren);
      }

      // DistinctScan operators have a single child called 'scan'
      else if (operatorName === "DistinctScan" && plan['scan']) {
        return(new PlanNode(convertN1QLPlanToPlanNodes(plan['scan'],null,lists),plan,null,lists.total_time));
      }

      // UNION operators will have an array of predecessors drawn from their "children".
      // we expect predecessor to be null if we see a UNION
      else if (operatorName === "UnionAll" && plan['~children']) {
        var unionChildren = [];

        // if there is a predecessor, it's probably an authorize node done before everything.
        // what to do? for now put it on every child of the Union

        for (var i = 0; i < plan['~children'].length; i++)
          unionChildren.push(convertN1QLPlanToPlanNodes(plan['~children'][i],predecessor,lists));

        var unionNode = new PlanNode(unionChildren,plan,null,lists.total_time);

        //if (predecessor)
        //  return(new PlanNode(predecessor,plan,[unionNode],lists.total_time));
        //else
          return(unionNode);
      }

      // NestedLoopJoin and NestedLoopNest operators have the INNER part of the join represented
      // by a ~child field which is a sequence of operators. The OUTER is the inputs to the
      // NestedJoin op, which are already captured

      else if ((operatorName === "NestedLoopJoin" || operatorName === "NestedLoopNest" ||
          operatorName === "HashJoin" || operatorName === "HashNest" ||
          operatorName === "Join" || operatorName === "Nest") && plan["~child"]) {
        //&& plan["~child"]["~children"]) {
        // do we have a
        var inner = predecessor;
        var outer = convertN1QLPlanToPlanNodes(plan['~child'],null,lists);
        return(new PlanNode([inner,outer],plan,null,lists.total_time));
      }

      // Similar to UNIONs, IntersectScan, UnionScan group a number of different scans
      // have an array of 'scan' that are merged together

      else if ((operatorName == "UnionScan") || (operatorName == "IntersectScan")) {
        var scanChildren = [];

        for (var i = 0; i < plan['scans'].length; i++)
          scanChildren.push(convertN1QLPlanToPlanNodes(plan['scans'][i],null,lists));

        return(new PlanNode(scanChildren,plan,null,lists.total_time));
      }

      // ignore FinalProject, IntermediateGroup, and FinalGRoup, which don't add anything

      else if (operatorName == "FinalProject" ||
          operatorName == "IntermediateGroup" ||
          operatorName == "FinalGroup") {
        return(predecessor);
      }

      // WITH operator has bindings, and a child

      else if (operatorName == "With") {
        var withNode = new PlanNode(predecessor,plan,null,lists.total_time);
        var withChildren = convertN1QLPlanToPlanNodes(plan['~child'],withNode,lists);
        return(withChildren);

      }

      // for all other operators, create a plan node
      else {
        return(new PlanNode(predecessor,plan,null,lists.total_time));
      }

    }


    //
    // structure analyzing explain plans. A plan is an object with an "#operator" field, and possibly
    // other fields depending on the operator, some of the fields may indicate child operators
    //

    function PlanNode(predecessor, operator, subsequence, total_query_time) {
      this.predecessor = predecessor; // might be an array if this is a Union node
      this.operator = operator;       // object from the actual plan
      this.subsequence = subsequence; // for parallel ops, arrays of plan nodes done in parallel
      //if (total_query_time && operator['#time_absolute'])
      //  this.time = Math.round(['#time_absolute']);
      if (total_query_time && operator['#time_absolute'])
        this.time_percent = Math.round(operator['#time_absolute']*1000/total_query_time)/10;
    }

    // how 'wide' is our plan tree?
    PlanNode.prototype.BranchCount = function() {
      if (this.predecessor == null)
        return(1);
      else {
        // our width is the max of the predecessor and the subsequence widths
        var predWidth = 0;
        var subsequenceWidth = 0;

        if (!_.isArray(this.predecessor))
          predWidth = this.predecessor.BranchCount();
        else
          for (var i=0; i < this.predecessor.length; i++)
            predWidth += this.predecessor[i].BranchCount();

        if (this.subsequence != null)
          subsequenceWidth = this.subsequence.BranchCount();

        if (subsequenceWidth > predWidth)
          return(subsequenceWidth);
        else
          return(predWidth);
      }
    }

    // how 'deep' is our plan tree?
    PlanNode.prototype.Depth = function() {
      var ourDepth = this.subsequence ? this.subsequence.Depth() : 1;

      if (this.predecessor == null)
        return(ourDepth);
      else if (!_.isArray(this.predecessor))
        return(ourDepth + this.predecessor.Depth());
      else {
        var maxPredDepth = 0;
        for (var i=0; i < this.predecessor.length; i++)
          if (this.predecessor[i].Depth() > maxPredDepth)
            maxPredDepth = this.predecessor[i].Depth();

        return(maxPredDepth + 1);
      }
    }

    //
    // get the user-visible name for a PlanNode
    //

    PlanNode.prototype.GetName = function() {
      // make sure we actually have a name
      if (!this.operator || !this.operator['#operator'])
        return(null);

      switch (this.operator['#operator']) {
      case "InitialProject": // we really want to all InitialProject just plain "Project"
        return("Project");

      case "InitialGroup":
        return("Group");

        // default: return the operator's name
      default:
        return(this.operator['#operator']);
      }
    }

    //
    // should the op be marked for:
    //  2) warning (probably expensive),
    //  1) attention (possibly expensive)
    //  0) don't mark
    //

    PlanNode.prototype.GetCostLevel = function() {
      var op = this.operator;
      // for now, the only unambiguously expensive operations are:
      // - PrimaryScan
      // - IntersectScan
      // we want to add correlated subqueries, but info on those in not yet
      // in the query plan. Other ops may be added in future.

      if (!op || !op['#operator'])
        return(0);

      switch (op['#operator']) {
      case "PrimaryScan":
      case "IntersectScan":
        return(2);

      }

      return(0);
    }

    //
    // get an HTML formatted string to use as a tooltip
    //

    PlanNode.prototype.GetTooltip = function() {
      var op = this.operator;

      if (!op || !op['#operator'])
        return("");

      // get details about the op, to see if we have info for a tool tip
      var childFields = getNonChildFieldList(op);
      if (childFields.length == 0) // no fields, no tool tip
        return("");

      // we have some results, build the tooltip
      var result = "";
      result += '<div class="row"><h5>' + op['#operator'] +
      '</h5></div><ul class="wb-explain-tooltip-list">';

      result += childFields;
      result += '</ul>';

      return(result);
    }

    // turn the fields of an operator into list elements,
    // but ignore child operators

    var childFieldNames = /#operator|\~child*|delete|update|scans|first|second|inputs/;

    function getNonChildFieldList(op) {
      var result = "";

      for (var field in op) if (!field.match(childFieldNames)) {
        var val = op[field];
        // add the field name as a list item
        result += '<li>' + field;

        // for a primitive value, just add that as well
        if (_.isString(val) || _.isNumber(val) || _.isBoolean(val))
          result += " - " + val;

        // if it's an array, create a sublist with a line for each item
        else if (_.isArray(val)) {
          result += '<ul>';
          for (var i=0; i<val.length; i++)
            if (_.isString(val[i]))
              result += '<li>' + val[i] + '</li>';
            else
              result += getNonChildFieldList(val[i]);
          result += '</ul>';
        }

        // if it's an object, have a sublist for it
        else if (_.isPlainObject(val)) {
          result += '<ul>';
          result += getNonChildFieldList(val);
          result += '</ul>';
        }
        result += '</li>';
      }
      return result;
    }

    //
    // get an array of node attributes that should be shown to the user
    //

    PlanNode.prototype.GetDetails = function() {
      var result = [];
      var op = this.operator;

      if (!op || !op['#operator'])
        return(result);

      // depending on the operation, extract different fields
      switch (op['#operator']) {

      case "IndexScan": // for index scans, show the keyspace
        pushTruncated(result,"by: " + op.keyspace + "." + op.index);
        break;

      case "IndexScan2":
      case "IndexScan3":
        pushTruncated(result,op.keyspace + "." + op.index);
        if (op.as)
          pushTruncated(result,"as: " + op.as);
        break;

      case "PrimaryScan": // for primary scan, show the index name
        pushTruncated(result,op.keyspace);
        break;

      case "InitialProject":
        pushTruncated(result,op.result_terms.length + " terms");
        break;

      case "Fetch":
        pushTruncated(result,op.keyspace + (op.as ? " as "+ op.as : ""));
        break;

      case "Alias":
        pushTruncated(result,op.as);
        break;

      case "NestedLoopJoin":
      case "NestedLoopNest":
      case "HashJoin":
      case "HashNest":
        pushTruncated(result,"on: " + op.on_clause);
        break;

      case "Limit":
      case "Offset":
        pushTruncated(result,op.expr);
        break;

      case "Join":
        pushTruncated(result,op.keyspace + (op.as ? " as "+op.as : "") + ' on ' + op.on_keys);
        break;

      case "Order":
        if (op.sort_terms) for (var i = 0; i < op.sort_terms.length; i++)
          pushTruncated(result,op.sort_terms[i].expr);
        break;

      case "InitialGroup":
      case "IntermediateGroup":
      case "FinalGroup":
        if (op.aggregates && op.aggregates.length > 0) {
          var aggr = "Aggrs: ";
          for (var i=0; i < op.aggregates.length; i++)
            aggr += op.aggregates[i];
          pushTruncated(result,aggr);
        }

        if (op.group_keys && op.group_keys.length > 0) {
          var keys = "By: ";
          for (var i=0; i < op.group_keys.length; i++)
            keys += op.group_keys[i];
          pushTruncated(result,keys);
        }
        break;

      case "Filter":
        if (op.condition)
          pushTruncated(result,op.condition);
        break;
      }

      // if there's a limit on the operator, add it here
      if (op.limit && op.limit.length)
        pushTruncated(result,op.limit);

      // if we get operator timings, put them at the end of the details
      if (op['#time_normal']) {

        pushTruncated(result,op['#time_normal'] +
            ((this.time_percent && this.time_percent > 0) ?
                ' (' + this.time_percent + '%)' : ''));
      }

      // if we have items in/out, add those as well
      if (op['#stats']) {
        var inStr = '';
        var outStr = '';

        // itemsIn is a number
        if (op['#stats']['#itemsIn'] || op['#stats']['#itemsIn'] === 0)
          inStr = op['#stats']['#itemsIn'].toString();
        if (op['#stats']['#itemsOut'] || op['#stats']['#itemsOut'] === 0)
          outStr = op['#stats']['#itemsOut'].toString();

        // if we have both inStr and outStr, put a slash between them
        var inOutStr = ((inStr.length > 0) ? inStr + ' in' : '') +
            ((inStr.length > 0 && outStr.length > 0) ? ' / ' : '') +
            ((outStr.length > 0) ? outStr + ' out' : '');

        if (inOutStr.length > 0)
          pushTruncated(result,inOutStr);
      }

      // handle Analytics operators
      if (op['variables'])
        pushTruncated(result,'vars: ' + op['variables']);
      if (op['expressions'])
        pushTruncated(result,'expr:' + op['expressions']);

      // all done, return the result
      return(result);
    }

    //
    // convenience function to ensure that items in the result array are no longer than a fixed length
    //

    var MAX_LENGTH = 35;

    function pushTruncated(array,item) {
      array.push(truncate(MAX_LENGTH,item));
    }

    //
    // truncate strings longer that a given length
    //

    function truncate(length, item) {
      if (!_.isString(item))
        return(item);

      if (item.length > length)
        return(item.slice(0,length) + "...");
      else
        return(item);
    }

    //
    // for debugging, this function prints out the plan to console.log
    //

    PlanNode.prototype.Print = function(indent) {
      var result = '';
      for (var i = 0; i < indent; i++)
        result += ' ';
      var opName = this.operator['#operator'];
      result += opName ? opName : "unknown op";
      result += " (" + this.BranchCount() + "," + this.Depth() + "), pred: " + this.predecessor;
      console.log(result);

      if (this.subsequence)
        this.subsequence.Print(indent + 2);

      if (this.predecessor)
        if (_.isArray(this.predecessor)) for (var i = 0; i < this.predecessor.length; i++) {
          result = '';
          for (var j = 0; j < indent+2; j++)
            result += ' ';
          console.log(result + "branch " + i)
          this.predecessor[i].Print(indent + 4);
        }
        else
          this.predecessor.Print(indent);
    }


    //
    // When we get a query plan, we want to create a list of buckets and fields referenced
    // by the query, so we can point out possible misspelled names
    //
    //   Sequence has '~children'
    //   Parallel has '~child'
    //   UnionAll has '~children'
    //   UnionScan/IntersectScan have 'scans'
    //   ExceptAll/IntersetAll have 'first' and 'second'
    //   DistinctScan has 'scan'
    //   Authorize has '~child'
    //   Merge has 'as', 'key', 'keyspace', 'delete' and 'update'


    function analyzePlan(plan, lists) {

      if (!lists)
        lists = {buckets : {}, fields : {}, indexes: {}, aliases: [], total_time: 0.0, warnings: []};

      // make

      if (!plan || _.isString(plan))
        return(null);

      // special case: prepared queries are marked by an "operator" field

      if (plan.operator)
        return(analyzePlan(plan.operator,null));

      // special case #2: plan with query timings is wrapped in an outer object
      if (plan.plan && !plan['#operator'])
        return(analyzePlan(plan.plan,null));

      //console.log("Inside analyzePlan: " + JSON.stringify(plan,null,true));

      // iterate over fields, look for "#operator" field
      var operatorName = plan['#operator'];
      //console.log("Analyzing plan node: " + operatorName);

      // some operators might have a warning status. Right now that is only
      // the NestedLoopJoin, which has "hint_not_followed"

      if (plan.hint_not_followed)
        lists.warnings.push("Hint not followed: " + plan.hint_not_followed);

      // at this point we should have an operation name and a field array
      //console.log("  after analyze, got op name: " + operatorName);
      // we had better have an operator name at this point

      if (!operatorName) {
        console.log("Error, no operator found for item: " + JSON.stringify(plan));
        return(lists);
      }
      //else
      //  console.log("Got operator: " + operatorName + ", stats: " + plan['#stats']);

      // if the operator has timing information, convert to readable and analyzable forms:
      if (plan['#time'] ||
          (plan['#stats'] && (plan['#stats'].execTime || plan['#stats'].servTime))) {
        var parsedValue = 0.0;
        if (plan['#time'])
          parsedValue = convertTimeStringToFloat(plan['#time']);
        else if (plan['#stats']) {
          if (plan['#stats'].execTime)
            parsedValue += convertTimeStringToFloat(plan['#stats'].execTime);
          if (plan['#stats'].servTime)
            parsedValue += convertTimeStringToFloat(plan['#stats'].servTime);
        }

        plan['#time_normal'] = convertTimeFloatToFormattedString(parsedValue);
        plan['#time_absolute'] = parsedValue;
        lists.total_time += parsedValue;
        //console.log("Got time:" + plan['#time'] + ", parsed: " + plan['#time_normal'] + ', abs: ' + plan['#time_absolute']);
      }


      // if we have a sequence, we analyze the children in order
      if (operatorName === "Sequence" && plan['~children']) {
        // a sequence may have aliases that rename buckets, but those aliases become invalid after
        // the sequence. Remember how long the sequence was at the beginning.
        var initialAliasLen = lists.aliases.length;

        for (var i = 0; i < plan['~children'].length; i++) {
          // if we see a fetch, remember the keyspace for subsequent projects
          if (plan['~children'][i]['#operator'] == "Fetch")
            lists.currentKeyspace = plan['~children'][i].keyspace;
          analyzePlan(plan['~children'][i], lists);
        }

        // remove any new aliases
        lists.aliases.length = initialAliasLen;
        return(lists);
      }

      // parallel groups are like sequences, but with only one child
      else if (operatorName === "Parallel" && plan['~child']) {
        analyzePlan(plan['~child'],lists);
        return(lists);
      }





      // Prepare operators have their plan inside prepared.operator
      else if (operatorName === "Prepare" && plan.prepared && plan.prepared.operator) {
        analyzePlan(plan.prepared.operator,lists);
        return(lists);
      }

      // ExceptAll and IntersectAll have 'first' and 'second' subqueries
      else if (operatorName === "ExceptAll" || operatorName === "IntersectAll") {
        if (plan['first'])
          analyzePlan(plan['first'],lists);

        if (plan['second'])
          analyzePlan(plan['second'],lists);

        return(lists);
      }

      // Merge has two children: 'delete' and 'update'
      else if (operatorName === "Merge") {
        if (plan.as)
          lists.aliases.push({keyspace: plan.keyspace, as: plan.as});

        if (plan['delete'])
          analyzePlan(plan['delete'],lists);

        if (plan['update'])
          analyzePlan(plan['update'],lists);

        if (plan.keyspace)
          getFieldsFromExpressionWithParser(plan.keyspace,lists);

        if (plan.key)
          getFieldsFromExpressionWithParser(plan.key,lists);

        return(lists);
      }

      // Authorize operators have a single child called '~child'
      else if (operatorName === "Authorize" && plan['~child']) {
        analyzePlan(plan['~child'],lists);
        return(lists);
      }

      // DistinctScan operators have a single child called 'scan'
      else if (operatorName === "DistinctScan" && plan['scan']) {
        analyzePlan(plan['scan'],lists);
        return(lists);
      }

      // Similar to UNIONs, IntersectScan, UnionScan group a number of different scans
      // have an array of 'scan' that are merged together

      else if ((operatorName == "UnionScan") || (operatorName == "IntersectScan")
          || (operatorName == "OrderedIntersectScan")) {
        for (var i = 0; i < plan['scans'].length; i++)
          analyzePlan(plan['scans'][i],lists);

        return(lists);
      }


      // UNION operators will have an array of predecessors drawn from their "children".
      // we expect predecessor to be null if we see a UNION
      else if ((operatorName == "Union" || operatorName === "UnionAll") && plan['~children']) {
        for (var i = 0; i < plan['~children'].length; i++)
          analyzePlan(plan['~children'][i],lists);

        return(lists);
      }

      //
      // The Order operator has an array of expressions
      //

      else if (operatorName == "Order") for (var i = 0; i < plan.sort_terms.length; i++) {
        getFieldsFromExpressionWithParser(plan.sort_terms[i].expr,lists);
      }


      //
      // the CreateIndex operator has keys, which are expressions we need to parse
      //

      else if (operatorName == "CreateIndex") {
        if (plan.keys && plan.keys.length)
          // CreateIndex keys are un-parsed N1QL expressions, we need to parse
          for (var i = 0; i < plan.keys.length; i++) {
            var parseTree = n1ql.parse(plan.keys[i].expr);

            // parse tree has array of array of strings, we will build
            if (parseTree && plan.keyspace) for (var p=0;p<parseTree.length; p++) {
              for (var j=0; j<parseTree[p].pathsUsed.length; j++) {
                if (parseTree[p].pathsUsed[j]) {
                  var field = plan.keyspace;
                  for (var k=0; k<parseTree[p].pathsUsed[j].length; k++) {
                    field += "." + parseTree[p].pathsUsed[j][k];
                  }

                  lists.fields[field] = true;
                }
              }
            }
          }
      }

      //

      // for all other operators, certain fields will tell us stuff:
      //  - keyspace is a bucket name
      //  - index is an index name
      //  - condition is a string containing an expression, fields there are of the form (`keyspace`.`field`)
      //  - expr is the same as condition
      //  - on_keys is an expression
      //  - limit is an expression
      //  - group_keys is an array of fields

      if (plan.keyspace)
        lists.buckets[plan.keyspace] = true;
      if (plan.index && plan.keyspace)
        lists.indexes[plan.keyspace + "." + plan.index] = true;
      else if (plan.index)
        lists.indexes[plan.index] = true;
      if (plan.group_keys) for (var i=0; i < plan.group_keys.length; i++)
        lists.fields[plan.group_keys[i]] = true;
      if (plan.condition)
        getFieldsFromExpressionWithParser(plan.condition,lists);
      if (plan.expr)
        getFieldsFromExpressionWithParser(plan.expr,lists);
      if (plan.on_keys)
        getFieldsFromExpressionWithParser(plan.on_keys,lists);
      if (plan.limit)
        getFieldsFromExpressionWithParser(plan.limit,lists);

      if (plan.as && plan.keyspace) {
        lists.aliases.push({keyspace: plan.keyspace, as: plan.as});
        //console.log("Got alias " + plan.as + " for " + plan.keyspace);
      }
      if (plan.result_terms && _.isArray(plan.result_terms))
        for (var i=0; i< plan.result_terms.length; i++) if (plan.result_terms[i].expr )
          if (plan.result_terms[i].expr == "self" && plan.result_terms[i].star &&
              lists.currentKeyspace)
            lists.fields[lists.currentKeyspace + '.*'] = true;
          else {
            getFieldsFromExpressionWithParser(plan.result_terms[i].expr,lists);
          }

      return(lists);
    }

    //
    // pull bucket and field names out of arbitrary expressions
    //
    // field names are expressed in nested parens, the simplest case is:
    //    (`bucket`.`field`)
    // but if there is a subfield, it looks like:
    //    ((`bucket`.`field`).`subfield`)
    // and array references are of the form:
    //    (((`bucket`.`field`)[5]).`subfield`)
    //
    // we need to work inside out, pulling out the bucket name and initial
    // field, then building as we go out.
    //

    function getFieldsFromExpressionWithParser(expression,lists) {

      //console.log("Got expr: "+ expression);

      var parseTree = n1ql.parse(expression);

      // parse tree has array of array of strings, we will build
      if (parseTree) for (var p=0;p<parseTree.length; p++) {
        if (parseTree[p].pathsUsed) for (var j=0; j<parseTree[p].pathsUsed.length; j++) {
          //console.log("Got path p: " + p + ", j: " + j + ", " + JSON.stringify(parseTree[p].pathsUsed[j]));
          if (parseTree[p].pathsUsed[j]) {
            var field = "";
            for (var k=0; k<parseTree[p].pathsUsed[j].length; k++) {
              var pathElement = parseTree[p].pathsUsed[j][k];

              // check for bucket aliases
              if (k==0 && lists.aliases) for (var a=0; a<lists.aliases.length; a++)
                if (lists.aliases[a].as === pathElement) {
                  pathElement = lists.aliases[a].keyspace;
                  break;
                }

              // first item in path should go into buckets
              if (k==0)
                lists.buckets[pathElement] = true;

              field += ((k > 0 && pathElement !== "[]") ? "." : "") + pathElement;
            }

            //console.log("  Got field: " + field);
            if (k > 1)
              lists.fields[field] = true;
          }
        }
      }
    }

    //
    // convert a duration expression, which might be 3m23.7777s or 234.9999ms or 3.8888s
    // or even 44.999us, to a real time value
    //

    function convertTimeToNormalizedString(timeValue)
    {
      var timeNumber = convertTimeStringToFloat(timeValue);
      return(convertTimeFloatToFormattedString(timeNumber));
    }

    //
    // convert a duration expression, which might be 3m23.7777s or 234.9999ms or 3.8888s
    // or even 44.999us, to a real floating point value in seconds
    //

    function convertTimeStringToFloat(timeValue)
    {
      // regex for parsing time values like 3m23.7777s or 234.9999ms or 3.8888s
      // groups: 1: minutes, 2: secs, 3: millis, 4: microseconds
      var durationExpr = /(?:(\d+)h)?(?:(\d+)m)?(?:(\d+\.\d+)s)?(?:(\d+\.\d+)ms)?(?:(\d+\.\d+)µs)?/;
      var result = 0.0;

      var m = timeValue.match(durationExpr);
      //console.log(m[0]);

      if (m) {
        // minutes
        if (m[1]) // hours value, should be an int
          result += parseInt(m[1])*3600;

        if (m[2]) // minutes value, should be an int
          result += parseInt(m[2])*60;

        // seconds
        if (m[3])
          result += parseFloat(m[3]);

        // milliseconds
        if (m[4])
          result += parseFloat(m[4])/1000;

        // ooh, microseconds!
        if (m[5])
          result += parseFloat(m[5])/1000000;
      }

      return(result);
    }

    //
    // take a floating point number of seconds and convert it to
    // 00:00.00000
    //

    function convertTimeFloatToFormattedString(timeValue) {
      var minutes = 0;
      if (timeValue > 60)
        minutes = Math.floor(timeValue/60);
      var seconds = timeValue - (minutes*60);

      var minutesStr = minutes.toString();
      if (minutesStr.length < 2)
        minutesStr = '0' + minutesStr;

      var secondsStr = (seconds < 10 ? '0' : '') + seconds.toString();
      if (secondsStr.length > 7)
        secondsStr = secondsStr.substring(0,6);

      return(minutesStr + ":" + secondsStr);
    }

    //
    //
    //
    // all done creating the service, now return it
    //

    return qwQueryPlanService;
  }



})();
