/**
 * Angular directive that takes a query plan data structure and renders it as
 * a D3 graph.
 */
/* global _, angular */
(function() {

  'use strict';
  angular.module('qwExplainVizD3',[]).directive('qwExplainVizD3', ['$compile', '$timeout', 'qwQueryService', getD3Explain]);

  var queryService = null;

  function getD3Explain($compile,$timeout,qwQueryService) {
    queryService = qwQueryService;
    return {
      restrict: 'A',
      scope: { data: '=qwExplainVizD3' },
      template: '<div></div>',
      link: function (scope, element) {

        scope.$watch('data', function (data) {
          // start with an empty div, if we have data convert it to HTML
          var content = "<div>{}</div>";
          if (data) {
            //console.log("Got data: " + JSON.stringify(data));
            content = "";

            if (_.isString(data))
              content = '<p class="text-small margin-left-half">' + data + '</p>';
            else if (_.isArray(data))
              content = '<p class="text-small margin-left-half">Graphical plans are not supported for multiple query sequences. Try the plan text view.</p>';
            else if (data["No data to display"] || data.errors)
              content = '<p class="text-small margin-left-half">' + JSON.stringify(data) + '</p>';

            // summarize plan in panel at the top
            if (data.analysis) {
              content += "<div class='row wb-explain-summary'>";
              if(data.mode === "analytics") {
                  content += constructDatasetsColumn(data.analysis.datasets);
              }
              content += "<div class='column'>";
              content += "<b>Indexes</b><br>";

              for (var f in data.analysis.indexes)
                if (f.indexOf("#primary") >= 0)
                  content += "<em class='cbui-plan-expensive'>" + f + "</em>&nbsp;&nbsp; ";
                else
                  content += "<em>" + f + "</em>&nbsp;&nbsp; ";
              content += "</div>";

              if(data.mode !== "analytics") {
                content += "<div class='column'>";
                content += "<b>Buckets</b><br>";
                for (var b in data.analysis.buckets)
                  content += "<em>" + b + "</em>&nbsp;&nbsp; ";
                content += "</div>";
              }

              content += "<div class='column'>";
              content += "<b>Fields</b><br>";
              for (var f in data.analysis.fields)
                content += "<em>" + f + "</em>&nbsp;&nbsp; ";
              content += "</div>";

// TBD: the selected orientation button should receive the "selected-orient" class, but it's not working
              content += "<div class='column row flex-grow-half flex-right'>";
              content += '<span ng-click="leftRight()" class="icon fa-caret-square-o-left wb-explain-plan-orient" ng-class="{\'wb-explain-plan-selected-orient\' : orientIs(1)}" title="change plan direction"></span>';
              content += '<span ng-click="rightLeft()" class="icon fa-caret-square-o-right wb-explain-plan-orient" ng-class="{\'wb-explain-plan-selected-orient\' : orientIs(3)}" title="change plan direction"></span>';
              content += '<span ng-click="bottomTop()" class="icon fa-caret-square-o-down wb-explain-plan-orient" ng-class="{\'wb-explain-plan-selected-orient\' : orientIs(4)}" title="change plan direction"></span>';
              content += '<span ng-click="topDown()" class="icon fa-caret-square-o-up wb-explain-plan-orient" ng-class="{\'wb-explain-plan-selected-orient\' : orientIs(2)}" title="change plan direction"></span>';
              content += "</div>";

              content += "<div class='column row flex-right'>";
              content += '<span ng-click="zoomIn()" class="icon fa-search-minus wb-explain-plan-zoom" title="zoom out - or use scroll wheel"></span>';
              content += '<span ng-click="zoomOut()" class="icon fa-search-plus wb-explain-plan-zoom" title="zoom in - or use scroll wheel"></span>';
              content += "</div>";

              content += "</div>";

              scope.topDown = topDown;
              scope.leftRight = leftRight;
              scope.bottomTop = bottomTop;
              scope.rightLeft = rightLeft;

              scope.orientIs = function(val) {return queryService.query_plan_options.orientation == val;};

              scope.zoomIn = zoomIn;
              scope.zoomOut = zoomOut;
            }
          }

          // set our element to use this HTML
          element.empty();

          if (content.length > 0) {
            var header = angular.element(content);
            $compile(header)(scope, function(compiledHeader) {element.append(compiledHeader)});

            // now add the d3 content

            if (data && data.plan_nodes) {
              // put the SVG inside a wrapper to allow scrolling
              wrapperElement = angular.element('<div class="wb-explain-d3-wrapper"></div>');
              element.append(wrapperElement);
              simpleTree = makeSimpleTreeFromPlanNodes(data.plan_nodes,null,"null");

              // if we're creating the wrapper for the first time, allow a delay for it to get a size
              if ($('.wb-explain-d3-wrapper').height() && $('.wb-explain-d3-wrapper').height() > 50)
                makeTree();
              else
                $timeout(makeTree,100);
            }
          }
        });
      }
    };
  }

  //
  // global so we can rebuild the tree when necessary
  //

  var wrapperElement;
  var simpleTree; // underlying data

  function makeTree() {

    makeD3TreeFromSimpleTree(simpleTree);
  }

  //
  // handle zooming
  //

  var orientLR = 1;
  var orientTB = 2;
  var orientRL = 3;
  var orientBT = 4;

  function topDown() {changeOrient(orientTB);}
  function bottomTop() {changeOrient(orientBT);}
  function leftRight() {changeOrient(orientLR);}
  function rightLeft() {changeOrient(orientRL);}

  function changeOrient(newOrient) {
    wrapperElement.empty();
    queryService.query_plan_options.orientation = newOrient;
    makeTree();
  }

  // handle drag/zoom events

  function zoom() {
    //var scale = d3.event.scale,
    //    translation = d3.event.translate;
    //console.log("scale: " + JSON.stringify(scale));
    //console.log("translate: " + JSON.stringify(translation));
    d3.select(".drawarea")
        .attr("transform", "translate(" + zoomer.translate() + ")" +
              " scale(" + zoomer.scale() + ")");
  }

  function zoomIn()  {zoomButton(false);}
  function zoomOut() {zoomButton(true); }

  function zoomButton(zoom_in) {
    var scale = zoomer.scale(),
      center = [canvas_width / 2 , canvas_height / 2],
      extent = zoomer.scaleExtent(),
      translate = zoomer.translate(),
      x = translate[0], y = translate[1],
      factor = zoom_in ? 2 : 1/2,
      target_scale = scale * factor;

    //  If we're already at an extent, done
    if (target_scale === extent[0] || target_scale === extent[1]) { return false; }
    //  If the factor is too much, scale it down to reach the extent exactly
    var clamped_target_scale = Math.max(extent[0], Math.min(extent[1], target_scale));
    if (clamped_target_scale != target_scale){
      target_scale = clamped_target_scale;
      factor = target_scale / scale;
    }

    //  Center each vector, stretch, then put back
    x = (x - center[0]) * factor + center[0];
    y = (y - center[1]) * factor + center[1];

    //  Transition to the new view over 100ms
    d3.transition().duration(100).tween("zoom", function () {
      var interpolate_scale = d3.interpolate(scale, target_scale),
      interpolate_trans = d3.interpolate(translate, [x,y]);
      return function (t) {
        zoomer.scale(interpolate_scale(t))
        .translate(interpolate_trans(t));
        zoom();
      };
    })
    //.each("end", function(){
    //  if (pressed) zoomButton(zoom_in);
  //  })
    ;
  }

  //////////////////////////////////////////////////////////////////////////
  // make a d3 tree
  //

  // Magic numbers for layout
  var svg, g, zoomer;
  var svg_scale = 1.0;
  var lineHeight = 15;        // height for line of text, on which node height is based
  var minNodeWidthVert = 155; // horizontal spacing for vertical trees
  var minNodeWidth = 225;     // horizontal spacing for horizontal trees

  var canvas_width, canvas_height;

  function makeD3TreeFromSimpleTree(root) {
    var duration = 500,
      i = 0;
    var vert = (queryService.query_plan_options.orientation == orientTB ||
        queryService.query_plan_options.orientation == orientBT);

    canvas_width = $('.wb-explain-d3-wrapper').width();
    canvas_height =  $('.wb-explain-d3-wrapper').height();

    //console.log("Svg width: " + canvas_width + ", height: " + canvas_height);
    //console.log("Got expected width: " + width + ", height: " + height);

    svg = d3.select(wrapperElement[0]).append('svg:svg')
//        .attr("width", canvas_width)
//        .attr("height", canvas_height)
        .attr("width", "100%")
        .attr("height", "100%")
        .attr("id", "outer_svg")
        .style("overflow", "scroll")
         .on("click",removeAllTooltips)

       .append("svg:g")
          .attr("class","drawarea")
          .attr("id", "svg_g")
      ;

    const trans = svg.transition().duration(duration);

    // need a definition for arrows on lines
    var arrowhead_refX = vert ? 0 : 0;
    var arrowhead_refY = vert ?  2 : 2;

    svg.append("defs").append("marker")
    .attr("id", "arrowhead")
    .attr("viewBox", "0 0 10 12")
    .attr("refX", arrowhead_refX) /*must be smarter way to calculate shift*/
    .attr("refY", arrowhead_refY)
    .attr("markerWidth", 20)
    .attr("markerHeight", 20)
    .attr("orient", "auto")
    .append("path")
    .attr("d", "M 6 0 V 4 L 0 2 Z"); //this is actual shape for arrowhead

    // minimum fixed sizes for nodes, depending on orientation, to prevent overlap
    var minNodeSize = vert ? [minNodeWidthVert,lineHeight*7] : [lineHeight*6,minNodeWidth];

    var tree = d3.layout.cluster()
    //.size([height, width])
    .nodeSize(minNodeSize) // give nodes enough space to avoid overlap
    ;

    // use nice curves between the nodes
    var diagonal = getLink();

    // assign nodes and links
    var nodes = tree.nodes(root);
    var links = tree.links(nodes);

    //
    // we want to pan/zoom so that the whole graph is visible.
    //
    // for some reason I can't get getBBox() to give me the bounding box for the overall
    // graph, so instead I'll just check the coords of all the nodes to get a bounding box
    //

    var minX = canvas_width, maxX = 0, minY = canvas_height, maxY = 0;
    nodes.forEach(function(d)
        {minX = Math.min(d.x,minX); minY = Math.min(d.y,minY); maxX = Math.max(d.x,maxX);maxY = Math.max(d.y,maxY);});
    //console.log("Actual width: " + (maxX - minX) + ", height: " + (maxY - minY));

    // to make a horizontal tree, x and y are swapped
    var dx = (vert ? maxX - minX : maxY - minY);
    var dy = (!vert ? maxX - minX : maxY - minY);
    var x = (vert ? (minX + maxX)/2 : (minY + maxY)/2);
    var y = (!vert ? (minX + maxX)/2 : (minY + maxY)/2);

    // if flipped, we need to flip the bounding box
    if (queryService.query_plan_options.orientation == orientBT)
      y = -y;
    else if (queryService.query_plan_options.orientation == orientRL)
      x = -x;

    var scale = Math.max(0.15,Math.min(2,.85 / Math.max(dx / canvas_width, dy / canvas_height)));
    var translate = [canvas_width / 2 - scale * x, canvas_height / 2 - scale * y];

    //console.log("Got new scale: " + scale + ", translate: " + JSON.stringify(translate));

    // set up zooming, including initial values to show the entire graph
    d3.select(".drawarea")
    .attr("transform","translate(" + translate + ")scale(" + scale + ")")
    ;

    zoomer = d3.behavior.zoom().scaleExtent([0.1, 2.5]).on("zoom", zoom);
    zoomer.translate(translate);
    zoomer.scale(scale);

    d3.select("svg").call(zoomer);

    // Each node needs a unique id. If id field doesn't exist, use incrementing value
    var node = svg.selectAll("g.node")
      .data(nodes, function(d) { return d.id || (d.id = ++i); });

    // Enter any new nodes at the parent's previous position.
    var nodeEnter = node.enter().append("svg:g")
    .attr("class", "wb-explain-node")
    .attr("transform", getRootTranslation)
    .on("click", makeTooltip);

// *** node drop-shadows come from this filter ******************
    // filters go in defs element
    var defs = svg.append("defs");

    // create filter with id #drop-shadow
    // height=130% so that the shadow is not clipped
    var filter = defs.append("filter")
        .attr("id", "drop-shadow")
        .attr("height", "130%");

    // SourceAlpha refers to opacity of graphic that this filter will be applied to
    // convolve that with a Gaussian with standard deviation 1 and store result
    // in blur
    filter.append("feGaussianBlur")
        .attr("in", "SourceAlpha")
        .attr("stdDeviation", 1)
        .attr("result", "blur");

    // translate output of Gaussian blur downwards with 1px
    // store result in offsetBlur
    filter.append("feOffset")
        .attr("in", "blur")
        .attr("dx", 0)
        .attr("dy", 1)
        .attr("result", "offsetBlur");

    // overlay original SourceGraphic over translated blurred opacity by using
    // feMerge filter. Order of specifying inputs is important!
    var feMerge = filter.append("feMerge");

    feMerge.append("feMergeNode")
        .attr("in", "offsetBlur")
    feMerge.append("feMergeNode")
        .attr("in", "SourceGraphic");

    // ********* create node style from data *******************
    nodeEnter.append("rect")
    .attr("width", function(d) {return getWidth(d);})
    .attr("height", function(d) {return getHeight(d);})
    .attr("rx", lineHeight) // sets corner roundness
    .attr("ry", lineHeight)
    .attr("x", function(d) {return(-1/2*getWidth(d))}) // make the rect centered on our x/y coords
    .attr("y", function(d) {return getHeight(d)*-1/2})
    .attr("class", function(d) { return d.level; })
    // drop-shadow filter
    .style("filter", "url(#drop-shadow)");


    nodeEnter.append("text")
    .attr("dy", function(d) {return getHeight(d)*-1/2 + lineHeight}) // m
    .attr("class", "wb-explain-node-text")
    .text(function(d) { return d.name })
    ;

    // handle up to 4 lines of details
    for (var i=0;i<4;i++) nodeEnter.append("text")
    .attr("dy", function(d) {return getHeight(d)*-1/2 + lineHeight*(i+2)})
    .attr("class", "wb-explain-node-text-details")
    .text(function(d) { return d.details[i] })
    ;

    // Transition nodes to their new position.
    var nodeUpdate = nodeEnter.transition(trans)
      .attr("transform", getNodeTranslation);

    //Transition exiting nodes to the parent's new position.
    var nodeExit = node.exit().transition(trans)
      .attr("transform", function(d) { return "translate(" + root.y + "," + root.x + ")"; })
      .remove();

    nodeExit.select("rect")
    .attr("r", 1e-6);

    // Update the links…
    var link = svg.selectAll("path.link")
    .data(links, function(d) { return d.target.id; });

    // Enter any new links at the parent's previous position.
    link.enter().insert("path", "g")
    //.attr("class", "wb-explain-link")
    .attr("class", function(l) { // clone nodes are duplicates. if we have a link between 2, we need to hide it
      if (l.target.cloneOf && l.source.cloneOf)
        return("wb-clone-link");
      else
        return("wb-explain-link");
    })
    .attr("marker-start", "url(#arrowhead)")
    .attr("d", function(d) {
      var o = {x: root.x0, y: root.y0};
      return diagonal({source: o, target: o});
    })
    // Transition links to their new position.
    .transition(trans)
    .attr("d", diagonal);

    // Transition exiting nodes to the parent's new position.
   link.exit().transition(trans)
    .attr("d", function(d) {
      var o = {x: root.x, y: root.y};
      return diagonal({root: o, target: o});
    })
    .remove();

    // Stash the old positions for transition.
    nodes.forEach(function(d) {
      d.x0 = d.x;
      d.y0 = d.y;
    });
  }

  //////////////////////////////////////////////////////////////////////////////////
  // layout/size functions that differ based on the orientation of the tree
  //

  function getLink() {
    switch (queryService.query_plan_options.orientation) {
    case orientTB:
      return d3.linkVertical()
        .x(function(node) {return(node.x)})
        .y(function(node) {return(node.y + getHeight(node)/2)});

    case orientBT:
      return d3.linkVertical()
      .x(function(node) {return(node.x)})
      .y(function(node) {return(-node.y - getHeight(node)/2)});

    case orientLR:
      return d3.linkHorizontal()
      .x(function(node) {return(node.y + getWidth(node)/2)})
      .y(function(node) {return(node.x)});

    case orientRL:
    default:
      return d3.linkHorizontal()
      .x(function(node) {return(-node.y - getWidth(node)/2)})
      .y(function(node) {return(node.x)});
    }
  }

  function getCloneConnectionEndPoint(node) {

    // otherwise base it on the graph orientation.
    switch (queryService.query_plan_options.orientation) {
    case orientTB:
      return [node.x , node.y];

    case orientBT:
      return [node.x , -node.y];

    case orientLR:
      return [node.y , node.x ];

    case orientRL:
    default:
      return [-node.y , node.x ];
    }
  }


  function getRootTranslation(root) {
    switch (queryService.query_plan_options.orientation) {
    case orientTB:
    case orientBT:
      root.x0 = 50;
      root.y0 = 0;
      break;

    case orientLR:
    case orientRL:
    default:
      root.x0 = 50;
      root.y0 = 0;
      break;
    }
    return "translate(" + root.x0 + "," + root.y0 + ")";
  }

  function getNodeTranslation(node) {
    // if the node is a clone, get the translation from the source
    if (node.cloneOf)
      return(getNodeTranslation(node.cloneOf));

    // otherwise base it on the orientation of the graph
    switch (queryService.query_plan_options.orientation) {
    case orientTB:
      return "translate(" + node.x + "," + node.y + ")";

    case orientBT:
      return "translate(" + node.x + "," + -node.y + ")";

    case orientLR:
      return "translate(" + node.y + "," + node.x + ")";

    case orientRL:
    default:
      return "translate(" + -node.y + "," + node.x + ")";
    }
  }

  //
  // make the tooltip for the given node. We want the tooltip
  // to go away when the click to dismiss
  //

  function makeTooltip(d) {
    removeAllTooltips();

    // the tooltip is relative to the query plan div, so we need to know its offset.
    var query_plan_offset = $(".wb-results-explain").offset();

    // create the new tooltip
    var tooltip_div = d3.select(".wb-results-explain")
    .append("div")
    .attr("id", "svg_tooltip" + d.id)
    .attr("class", "wb-explain-tooltip")
    //.on("click", function(event) {
    //  return tooltip_div.style("display", "none");
    //})
    ;

    // get the approximate linecount by looking at HTML elements
    //var lines = d.tooltip ? (d.tooltip.match(/<h5>|<li>/gi) || []).length : 0;

    if (d.tooltip && d.tooltip.length > 0) {
    tooltip_div.transition().duration(300).style("display", "block");
    var header_div = tooltip_div.append("div");
    header_div.html('<a class="ui-dialog-titlebar-close modal-close" onclick="console.log(\"click\")"> X </a>');
    tooltip_div.html(d.tooltip)
    .style("left", (d3.event.x + 40 - query_plan_offset.left) + "px")
    .style("top", (d3.event.y - query_plan_offset.top/2) + "px");
    }

    d3.event.stopPropagation();
  }

  function removeAllTooltips() {
    // get rid of any existing tooltips
    d3.select(".wb-results-explain").selectAll('.wb-explain-tooltip').remove();
  }

  //
  // function to compute height for a given node based on how many lines
  // of details it has
  //

  function getHeight(node) {
    var numLines = 2;
    if (node.details)
      numLines += node.details.length;
    return(lineHeight*numLines);
  }

  //
  // compute width by finding the longest line, counting characters
  //

  function getWidth(node) {
    var maxWidth = 20; // leave at least this much space
    if (node.name && node.name.length > maxWidth)
      maxWidth = node.name.length;
    if (node.details)
      for (var i=0; i < node.details.length; i++)
        if (node.details[i] && node.details[i].length > maxWidth)
          maxWidth = node.details[i].length;

    return(maxWidth * 5); //allow 5 units for each character
  }

  //
  // recursively turn the tree of ops into a simple tree to give to d3
  //

  function makeSimpleTreeFromPlanNodes(plan,next,parent,nodeCache) {
    // keep a cache of nodes we have created, in case we see them again
    if (!nodeCache)
      nodeCache = {};

    // we ignore operators of nodes with subsequences, and put in the subsequence
    if (plan.subsequence)
      return(makeSimpleTreeFromPlanNodes(plan.subsequence,plan.predecessor,parent,nodeCache));

    if (!plan || !plan.operator)
      return(null);

    // otherwise build a node based on this operator
    var opName = (plan && plan.operator && plan.operator['#operator'])
      ? plan.operator['#operator'] : "unknown op";

    var result = {
        name: plan.GetName(),
        details: plan.GetDetails(),
        parent: parent,
        children: [],
        level: "node", // default background color
        time: plan.time,
        time_percent: plan.time_percent,
        tooltip: plan.GetTooltip()
    };

    // how expensive are we? Color background by cost, if we know
    if (plan && plan.time_percent) {
      if (plan.time_percent >= 20)
        result.level = "wb-explain-node-expensive-1";
      else if (plan.time_percent >= 5)
        result.level = "wb-explain-node-expensive-2";
      else if (plan.time_percent >= 1)
        result.level = "wb-explain-node-expensive-3";
    }

    // if we have seen a node with this id before, we need to mark the new node as a clone
    // the clone will then override the layout to move to it's twin's location
    // this is because D3 doesn't support DAGs, so we will fake it with a tree with co-located nodes

    if (plan && plan.operator && plan.operator.operatorId && nodeCache[plan.operator.operatorId])
      result.cloneOf = nodeCache[plan.operator.operatorId];

    // otherwise add this new node to the cache
    else if (plan.operator.operatorId)
      nodeCache[plan.operator.operatorId] = result;

    // if the plan has a 'predecessor', it is either a single plan node that should be
    // our child, or an array marking multiple children

    if (plan.predecessor)
      if (!_.isArray(plan.predecessor))
        result.children.push(makeSimpleTreeFromPlanNodes(plan.predecessor,next,result.name,nodeCache));
      else for (var i=0; i< plan.predecessor.length; i++)
        result.children.push(makeSimpleTreeFromPlanNodes(plan.predecessor[i],null,result.name,nodeCache));

    return(result);
  }

  function constructDatasetsColumn(datasets) {
      var html = "<div class='column'>";
      html += "<b>Datasets</b><br>";
      for (var dataset in datasets) {
          html += "<em>" + dataset + "</em>&nbsp;&nbsp; ";
      }
      html += "</div>";
      return html;
  }

})();
