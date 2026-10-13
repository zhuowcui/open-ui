import unittest

from tools.wpt import native_mutation_codegen as native
from tools.wpt import port_wpt as porter


class NativeMutationReplayTests(unittest.TestCase):
    def compile(self, markup, script):
        source = markup + '<script>' + script + '</script>'
        ir = native.inventory(source, 'pending/unit')
        return ir, native.ReplayCompiler(source, ir).compile('replay_unit')[0]

    def test_prepend_is_recorded_after_create_and_detached_style(self):
        ir, code = self.compile('<div id="parent"><div id="old"></div></div>',
            "document.body.offsetHeight; const child = document.createElement('div');"
            "child.style.height = '2em'; document.getElementById('parent').prepend(child);")
        self.assertEqual([o['kind'] for o in ir['operations']],
                         ['layout-barrier', 'create-element', 'set-style', 'prepend'])
        self.assertLess(code.index('engine.update()?'), code.index('"2em"'))
        self.assertLess(code.index('"2em"'), code.index('engine.insert_before('))
        self.assertIn('engine.append_or_move_child(', code)

    def test_empty_inline_display_reveals_stylesheet_and_calls_remove(self):
        ir, code = self.compile('<style>#target { display:flex }</style>'
            '<div id="target" style="display:none"></div>',
            "let target = document.getElementById('target');"
            "document.body.offsetHeight; target.style.display = '';")
        self.assertEqual(ir['operations'][-1]['kind'], 'remove-style')
        removal = code.index('engine.remove_property(node_1, StyleProperty::Display)')
        self.assertGreater(code.rindex('StyleProperty::Display, "flex"'), removal)
        self.assertNotIn('StyleProperty::Display, ""', code)

    def test_barriers_remain_between_authored_transform_assignments(self):
        ir, code = self.compile('<div id="target" style="transform:scale(1)"></div>',
            "document.body.offsetTop; document.querySelector('#target').style.transform='scale(1.5)';"
            "document.body.offsetTop; document.querySelector('#target').style.transform='scale(4)';")
        self.assertEqual([o['kind'] for o in ir['operations']],
                         ['layout-barrier', 'set-style', 'layout-barrier', 'set-style'])
        first = code.index('// AST operation 0')
        second = code.index('// AST operation 2')
        self.assertLess(first, code.index('"scale(1.5)"'))
        self.assertLess(code.index('"scale(1.5)"'), second)
        self.assertLess(second, code.index('"scale(4)"'))
        self.assertEqual(code.count('let _ = engine.update()?'), 2)

    def test_unhandled_calls_cannot_be_silently_lowered(self):
        for call in ["document.execCommand('selectAll')", "window.parent.postMessage('x', '*')",
                     "target.scrollIntoView()", "unknown()"]:
            ir = native.inventory('<script>target.style.width="2px";' + call + ';</script>', 'bad')
            self.assertFalse(ir['lowerable'], call)
            self.assertTrue(ir['rejections'])

    def test_dynamic_input_control_and_receiver_fail_closed(self):
        for code in ["target.style.width = location.hash", "if (true) target.style.width='2px'",
                     "requestAnimationFrame(()=>target.style.width='2px')",
                     "document.body.appendChild(a,b)",
                     "fake.createElement('div')", "document.querySelector(location.hash).style.width='2px'"]:
            self.assertFalse(native.inventory('<script>' + code + '</script>', 'bad')['lowerable'])

    def test_reserved_and_repeated_bindings_fail_closed(self):
        for code in ["const document=1; document.body.style.width='2px';",
                     "var target=document.body; var target=document.body; target.style.width='2px';",
                     "const window=1; document.body.style.width='2px';"]:
            self.assertFalse(native.inventory('<script>' + code + '</script>', 'bad')['lowerable'])

    def test_inert_module_nomodule_and_external_scripts_fail_closed(self):
        for attrs in ['type="text/plain"', 'type="module"', 'nomodule',
                      'src="external.js"', 'async']:
            source = '<script ' + attrs + '>document.body.style.width="2px";</script>'
            ir = native.inventory(source, 'bad')
            self.assertFalse(ir['lowerable'], attrs)
            self.assertTrue(ir['rejections'])
        classic = native.inventory('<script type="text/javascript">'
            'document.body.style.width="2px";</script>', 'classic')
        self.assertTrue(classic['lowerable'])

    def test_stylesheet_important_winner_and_inline_priority_replacement(self):
        _ir, code = self.compile('<style>#target{color:red!important}</style>'
            '<div id="target" style="color:green!important"></div>',
            "document.getElementById('target').style.color='blue';")
        assignment = code.index('// AST operation 0')
        self.assertLess(code.index('StyleProperty::Color, "blue"', assignment),
                        code.rindex('StyleProperty::Color, "red"'))

    def test_unknown_ir_kind_order_and_target_fail_closed(self):
        ir = native.inventory('<script>target.style.width="2px";</script>', 'bad')
        for change in [{'kind': 'forgotten-effect'}, {'order': 2}]:
            altered = {**ir, 'operations': [{**ir['operations'][0], **change}]}
            with self.assertRaises(ValueError):
                native.ReplayCompiler('<div id="target"></div>', altered).compile('test')
        with self.assertRaises(ValueError):
            native.ReplayCompiler('<div></div>', ir).compile('test')

    def test_raw_units_currentcolor_and_percent_survive_lowering(self):
        _ir, code = self.compile('<div id="target" style="font-size:2em;width:25%"></div>',
            "let target=document.getElementById('target'); target.style.color='currentcolor';")
        self.assertIn('StyleProperty::FontSize, "2em"', code)
        self.assertIn('StyleProperty::Width, "25%"', code)
        self.assertIn('StyleProperty::Color, "currentcolor"', code)
        self.assertNotIn('install_derived_style', code)

    def test_css_effects_and_attribute_semantics_are_not_discarded(self):
        for markup in ['<style>@media screen { div { color:red } }</style><div id="target"></div>',
                       '<style media="print">div{color:red}</style><div id="target"></div>',
                       '<style>div:hover {color:red}</style><div id="target"></div>',
                       '<div id="target" dir="rtl"></div>',
                       '<div id="target" style="transition:all 1s"></div>',
                       '<div id="target" style="scrollbar-width:thin"></div>',
                       '<div id="target" style="background:url(x)"></div>']:
            with self.assertRaises(ValueError):
                self.compile(markup, "document.getElementById('target').style.width='2px';")

    def test_capture_scrollbar_policy_covers_initial_and_script_created_nodes(self):
        _ir, code = self.compile('<div id="target"></div>',
            "const node=document.createElement('span'); document.body.append(node);")
        self.assertEqual(code.count('create_capture_node(engine,'), 4)
        self.assertIn('StyleProperty::ScrollbarWidth, "none"', native.HEADER)
        with self.assertRaises(ValueError):
            self.compile('<div id="target"></div>',
                "document.getElementById('target').style.scrollbarWidth='thin';")

    def test_prepend_argument_order_and_cycle_validation(self):
        ir, code = self.compile('<div id="parent"></div>',
            "const a=document.createElement('div'); const b=document.createElement('span');"
            "document.getElementById('parent').prepend(a,b);")
        self.assertEqual([c['name'] for c in ir['operations'][-1]['children']], ['a', 'b'])
        self.assertLess(code.index('engine.insert_before(node_1, node_2'),
                        code.index('engine.insert_before(node_1, node_3'))
        with self.assertRaises(ValueError):
            self.compile('<div id="parent"></div>',
                "let a=document.getElementById('parent'); a.prepend(a);")

    def test_reparent_recascades_equal_values_in_changed_shorthand_order(self):
        _ir, code = self.compile('<style>.a .child{padding:10px;padding-left:20px}'
            '.b .child{padding-left:20px;padding:10px}</style>'
            '<div class="a"><div class="child" id="child"></div></div>'
            '<div class="b" id="b"></div>',
            "let child=document.getElementById('child'); document.getElementById('b').append(child);")
        moved = code[code.index('// AST operation 1'):]
        self.assertIn('engine.append_or_move_child(node_3, node_2)', moved)
        self.assertLess(moved.index('StyleProperty::PaddingLeft, "20px"'),
                        moved.index('StyleProperty::Padding, "10px"'))

    def test_shared_final_cascade_prepend_and_empty_display(self):
        ir = {'lowerable': True, 'operations': [
            {'order': 0, 'kind': 'set-style', 'target': {'kind': 'dom-query', 'method': 'getElementById', 'argument': 'p'},
             'cascade_delta': {'css_name': 'display', 'value': ''}},
            {'order': 1, 'kind': 'create-element', 'source_offset': 9, 'binding': 'new', 'tag_name': 'span'},
            {'order': 2, 'kind': 'prepend', 'target': {'kind': 'dom-query', 'method': 'getElementById', 'argument': 'p'},
             'children': [{'kind': 'binding', 'name': 'new'}]}]}
        parser = porter._parse_wpt_markup('<style>#p{display:flex}</style>'
            '<div id="p" style="display:none"><div id="old"></div></div>', str(native.ROOT), mutation_ir=ir)
        self.assertEqual(parser.root.children[0].styles['display'], 'flex')
        self.assertEqual([n.tag for n in parser.root.children[0].children], ['span', 'div'])

    def test_rust_literals_keep_unicode_and_escape_control_characters(self):
        self.assertEqual(native.rust_string('λ\n"\\'), '"λ\\n\\"\\\\"')
        self.assertEqual(native.rust_string('\x08'), '"\\u{8}"')

    def test_color_only_shorthand_keeps_order_and_important_winner(self):
        for declarations, winner in [
            ('background:red;background-color:blue', 'blue'),
            ('background-color:blue;background:red', 'red'),
            ('background:red!important;background-color:blue', 'red')]:
            _ir, code = self.compile('<div id="target" style="' + declarations + '"></div>',
                "document.getElementById('target').style.width='2px';")
            self.assertIn('StyleProperty::BackgroundColor, "' + winner + '"', code)


if __name__ == '__main__':
    unittest.main()
