# Colloid, virus and bacteria transport
keywords: colloids; viruses; bacteria; attachment; detachment; filtration theory; blocking; straining; ripening; random sequential adsorption; RSA; Langmuir

Colloids, viruses and bacteria are transported like solutes but are retained by kinetic attachment to the solid phase rather than by sorption. HYDRUS-1D (and Rusteau) uses two kinetic sites with attachment and detachment [@schijvenSimunek2002; @bradford2003]:

$$
∂(θ c)/∂t + ρ ∂s_{e}/∂t + ρ ∂s_{1}/∂t + ρ ∂s_{2}/∂t = ∂/∂x (θ D ∂c/∂x) − ∂(q c)/∂x − R
ρ ∂s_{i}/∂t = θ ψ_{i} k_{att,i} c − k_{det,i} ρ s_{i} − (decay)
$$

with the concentration c of the colloids in the liquid, the concentrations s_{1}, s_{2} attached to the two sites, the attachment and detachment coefficients k_{att,i}, k_{det,i} [1/T] and the dimensionless functions ψ_{i} that describe blocking, ripening or straining.

## Blocking and straining functions {#psi}

| iPsi | Function |
|---|---|
| 0 | no blocking, ψ = 1 |
| 1 | Langmuir blocking, ψ = 1 − s/s_{max} |
| 2 | ripening: attachment increases with the attached mass |
| 3 | random sequential adsorption (RSA) [@adamczyk1994; @johnsonElimelech1995] |
| 4 | depth-dependent straining, ψ = ((d_{c} + z − z_{0})/d_{c})^{−β} [@bradford2003] |
| 5 | straining combined with Langmuir blocking |

s_{max} (SMax) is the maximum solid-phase concentration of a site and d_{c} the diameter of the sand grains.

## Attachment from filtration theory {#filtration}

With *attachment rates from filtration theory*, the attachment coefficient follows from clean-bed filtration theory [@logan1995]:

$$ k_{att} = 3(1 − n) / (2 d_{c}) · η · α · v $$

with the porosity n, the collector (grain) diameter d_{c}, the single-collector efficiency η [@rajagopalanTien1976], the sticking efficiency α (entered in the column of the attachment coefficient) and the pore-water velocity v. The particle diameter d_{p} enters η.

## Parameters

The parameters are entered per solute (colloid) and material on the page [T / θ dependence, colloids](solute_advanced): rKa1, rKd1, SMax1, rKa2, rKd2, SMax2 and the blocking types or the diameters Dc, Dp.
