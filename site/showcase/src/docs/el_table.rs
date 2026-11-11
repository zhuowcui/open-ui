use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <table style:border-collapse="collapse" style:font-size="14px" style:width="420px">
            <thead>
                <tr style:border-bottom="1px solid #e4e4e7" style:color="#71717a">
                    <th style:text-align="left" style:padding="8px" style:font-weight="500">"Invoice"</th>
                    <th style:text-align="left" style:padding="8px" style:font-weight="500">"Status"</th>
                    <th style:text-align="right" style:padding="8px" style:font-weight="500">"Amount"</th>
                </tr>
            </thead>
            <tbody>
                <tr style:border-bottom="1px solid #e4e4e7">
                    <td style:padding="8px">"INV-001"</td>
                    <td style:padding="8px">"Paid"</td>
                    <td style:padding="8px" style:text-align="right">"$250.00"</td>
                </tr>
                <tr style:border-bottom="1px solid #e4e4e7">
                    <td style:padding="8px">"INV-002"</td>
                    <td style:padding="8px">"Pending"</td>
                    <td style:padding="8px" style:text-align="right">"$150.00"</td>
                </tr>
            </tbody>
            <tfoot>
                <tr style:font-weight="600">
                    <td colspan="2" style:padding="8px">"Total"</td>
                    <td style:padding="8px" style:text-align="right">"$400.00"</td>
                </tr>
            </tfoot>
        </table>
    }
}
